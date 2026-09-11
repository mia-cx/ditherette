//! Capacity-accounted three-row work storage preserving the frozen contribution order.

use std::mem::size_of;

use super::ErrorDiffusionKernel;
use crate::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageView, PaletteIndex8, Rgba8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{
                AlphaPolicy, Diffusion, DiffusionFeedback, DitherPolicy, MatchPolicy, Placement,
                MAX_SOURCE_SIDE,
            },
        },
        dither::placement::placement_mask_with_converter,
        palette::{allocation::Budget, PalettePixel, PreparationError, PreparedPalette},
        quantize::{
            matcher::{PaletteColor, PaletteMatcher},
            PreparedQuantizer,
        },
    },
};

const ROWS: usize = 3;

/// Native request diagnostics and allocation-free execution/preparation failures.
#[derive(Debug)]
pub enum DiffusionError {
    Request(crate::prod::contract::error::DitheretteError),
    Preparation(PreparationError),
    Execution(Failure),
}

/// Executes a borrowed native request with bounded work and fallibly reserved owned output.
pub fn diffuse(
    request: crate::prod::contract::request::DitherQuantizeRequest<'_>,
    memory_limit: u64,
) -> Result<IndexedImage, DiffusionError> {
    use crate::prod::{color::packed::Converter, contract::request::Request};
    let layout = Request::DitherAndQuantize(request)
        .validate()
        .map_err(DiffusionError::Request)?;
    let policy = DiffusionPolicy::new(request.dither).map_err(DiffusionError::Execution)?;
    let count = layout.output.pixel_count().expect("validated dimensions");
    let fixed = (size_of::<ImageBuf<PaletteIndex8>>()
        + size_of::<DiffusionPolicy>()
        + size_of::<Converter>()) as u64;
    let preparation_limit = memory_limit
        .checked_sub(fixed + count as u64)
        .ok_or(DiffusionError::Preparation(PreparationError::memory()))?;
    let mut prepared = PreparedDiffusion::try_new(
        layout.source.dimensions().width(),
        request.quantize.palette,
        request.quantize.alpha,
        request.quantize.matching,
        preparation_limit,
    )
    .map_err(DiffusionError::Preparation)?;
    let mut budget = Budget::new(memory_limit, fixed + prepared.capacity_bytes())
        .map_err(DiffusionError::Preparation)?;
    let mut indices = Vec::new();
    budget
        .reserve(&mut indices, count)
        .map_err(DiffusionError::Preparation)?;
    indices.resize(count, 0);
    prepared
        .execute(layout.source, &mut indices, policy)
        .map_err(DiffusionError::Execution)?;
    Ok(prepared.into_indexed(
        ImageBuf::from_vec_packed(indices, layout.output).expect("reserved output storage"),
    ))
}

/// Validated scalar scan controls. Matching and palette ownership stay in PreparedQuantizer.
#[derive(Clone, Copy)]
pub struct DiffusionPolicy {
    kernel: ErrorDiffusionKernel,
    feedback: DiffusionFeedback,
    strength: f32,
    serpentine: bool,
    placement: Placement,
}

impl DiffusionPolicy {
    /// Rejects invalid native controls before source copying or allocation.
    pub fn new(dither: DitherPolicy) -> Result<Self, Failure> {
        let DitherPolicy::Diffusion {
            kernel,
            feedback,
            strength,
            serpentine,
            placement,
        } = dither
        else {
            return Err(Failure::new(
                ErrorCode::UnsupportedOperation,
                ErrorPath::Dither,
            ));
        };
        nonnegative(strength, ErrorPath::DitherStrength)?;
        if let Placement::Adaptive {
            radius,
            threshold,
            softness,
        } = placement
        {
            if !(1..=MAX_SOURCE_SIDE).contains(&radius) {
                return Err(Failure::new(
                    ErrorCode::InvalidSettings,
                    ErrorPath::DitherRadius,
                ));
            }
            nonnegative(threshold, ErrorPath::DitherThreshold)?;
            nonnegative(softness, ErrorPath::DitherSoftness)?;
        }
        Ok(Self {
            kernel: match kernel {
                Diffusion::FloydSteinberg => ErrorDiffusionKernel::FloydSteinberg,
                Diffusion::Sierra => ErrorDiffusionKernel::Sierra,
                Diffusion::SierraLite => ErrorDiffusionKernel::SierraLite,
                Diffusion::Atkinson => ErrorDiffusionKernel::Atkinson,
            },
            feedback,
            strength,
            serpentine,
            placement,
        })
    }
}

fn nonnegative(value: f32, path: ErrorPath) -> Result<(), Failure> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(Failure::new(ErrorCode::InvalidSettings, path))
    }
}

/// Source-initialized work rows retain every f64 contribution followed by f32 rounding.
/// Accumulating errors separately and adding source color later would change this recipe.
pub struct PreparedDiffusion {
    quantizer: PreparedQuantizer,
    work: Vec<[f32; 3]>,
}

impl PreparedDiffusion {
    /// Counts the record, landed quantizer, and three rows before any allocation.
    pub fn required_capacity_bytes(
        width: u32,
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        matching: MatchPolicy,
    ) -> Result<u64, PreparationError> {
        Ok(
            PreparedQuantizer::required_capacity_bytes(entries, alpha, matching)?
                + (size_of::<Self>() - size_of::<PreparedQuantizer>()) as u64
                + u64::from(width) * ROWS as u64 * size_of::<[f32; 3]>() as u64,
        )
    }

    /// Reserves all work and palette capacities fallibly within the caller's remaining budget.
    pub fn try_new(
        width: u32,
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        limit: u64,
    ) -> Result<Self, PreparationError> {
        let required = Self::required_capacity_bytes(width, entries, alpha, matching)?;
        if required > limit {
            return Err(PreparationError::memory());
        }
        let work_len = (width as usize)
            .checked_mul(ROWS)
            .ok_or_else(PreparationError::memory)?;
        let overhead = (size_of::<Self>() - size_of::<PreparedQuantizer>()) as u64;
        let work_bytes = work_len as u64 * size_of::<[f32; 3]>() as u64;
        let quantizer =
            PreparedQuantizer::try_new(entries, alpha, matching, limit - overhead - work_bytes)?;
        let mut budget = Budget::new(limit, overhead + quantizer.capacity_bytes())?;
        let mut work = Vec::new();
        budget.reserve(&mut work, work_len)?;
        work.resize(work_len, [0.0; 3]);
        Ok(Self { quantizer, work })
    }

    /// Actual work capacity alone depends on width, regardless of source height.
    pub fn scratch_capacity_bytes(&self) -> u64 {
        (self.work.capacity() * size_of::<[f32; 3]>()) as u64
    }

    /// Actual owned record, quantizer tables, palette metadata, and work capacities.
    pub fn capacity_bytes(&self) -> u64 {
        (size_of::<Self>() - size_of::<PreparedQuantizer>()) as u64
            + self.quantizer.capacity_bytes()
            + self.scratch_capacity_bytes()
    }

    pub fn palette(&self) -> &PreparedPalette {
        self.quantizer.palette()
    }

    /// Moves the prepared metadata into durable native output without allocation.
    pub fn into_indexed(self, indices: ImageBuf<PaletteIndex8>) -> IndexedImage {
        self.quantizer.into_indexed(indices)
    }

    /// Executes without allocation. The source is unchanged and fixed-index pixels remain sinks.
    pub fn execute(
        &mut self,
        source: ImageView<'_, Rgba8>,
        indices: &mut [u8],
        policy: DiffusionPolicy,
    ) -> Result<(), Failure> {
        execute_with_scratch(&self.quantizer, &mut self.work, source, indices, policy)
    }
}

/// Runs the existing row kernel with separately owned palette preparation and scratch.
pub(crate) fn execute_with_scratch(
    quantizer: &PreparedQuantizer,
    work: &mut [[f32; 3]],
    source: ImageView<'_, Rgba8>,
    indices: &mut [u8],
    policy: DiffusionPolicy,
) -> Result<(), Failure> {
    execute_with_progress(quantizer, work, source, indices, policy, |_| Ok(()))
}

/// Reports completed rows without resetting or replaying the continuous feedback traversal.
pub(crate) fn execute_with_progress(
    quantizer: &PreparedQuantizer,
    work: &mut [[f32; 3]],
    source: ImageView<'_, Rgba8>,
    indices: &mut [u8],
    policy: DiffusionPolicy,
    progress: impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    BorrowedDiffusion { quantizer, work }.execute(source, indices, policy, progress)
}

struct BorrowedDiffusion<'a> {
    quantizer: &'a PreparedQuantizer,
    work: &'a mut [[f32; 3]],
}

impl BorrowedDiffusion<'_> {
    fn execute(
        &mut self,
        source: ImageView<'_, Rgba8>,
        indices: &mut [u8],
        policy: DiffusionPolicy,
        mut progress: impl FnMut(u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        let width = source.dimensions().width_usize();
        let height = source.dimensions().height_usize();
        assert_eq!(self.work.len(), width * ROWS);
        assert_eq!(indices.len(), width * height);
        let palette = self.quantizer.palette();
        let fixed_alpha = if palette.visible.is_empty() {
            let PalettePixel::Index(index) = palette.prepare_pixel([0; 4]) else {
                unreachable!("transparent-only palette has no visible matching");
            };
            Some((255, index))
        } else {
            palette.preserved_alpha()
        };
        for y in 0..height.min(ROWS) {
            self.fill_row(source, y, policy.feedback);
        }
        for y in 0..height {
            let reverse = policy.serpentine && y % 2 == 1;
            let row = source.row(y as u32).expect("validated source row");
            for step in 0..width {
                let x = if reverse { width - 1 - step } else { step };
                let offset = y * width + x;
                if let Some((_, index)) =
                    fixed_alpha.filter(|&(cutoff, _)| row[x * 4 + 3] <= cutoff)
                {
                    indices[offset] = index;
                    continue;
                }
                let current = self.work[(y % ROWS) * width + x];
                if current.iter().any(|value| !value.is_finite()) {
                    return Err(arithmetic(ErrorPath::DiffusionWork));
                }
                let matcher = self.quantizer.matcher();
                let (selected, error) = match policy.feedback {
                    DiffusionFeedback::SrgbBytes => {
                        let rgb = current
                            .map(|channel| f64::from(channel).round().clamp(0.0, 255.0) as u8);
                        let selected =
                            nearest_finite(matcher, self.quantizer.converter().coordinates(rgb))?;
                        let start = usize::from(selected.index) * 4;
                        let error: [f64; 3] = std::array::from_fn(|axis| {
                            f64::from(rgb[axis])
                                - f64::from(self.quantizer.palette().palette.rgba[start + axis])
                        });
                        (selected, error)
                    }
                    DiffusionFeedback::Matching => {
                        let selected = nearest_finite(matcher, current)?;
                        let error = std::array::from_fn(|axis| {
                            f64::from(current[axis]) - f64::from(selected.coordinates[axis])
                        });
                        (selected, error)
                    }
                };
                indices[offset] = selected.index;
                let mask = placement_mask_with_converter(
                    source,
                    x as u32,
                    y as u32,
                    matcher.matching.space(),
                    policy.placement,
                    self.quantizer.converter(),
                );
                let strength_mask = f64::from(policy.strength) * f64::from(mask);
                for tap in policy.kernel.taps() {
                    let dx = if reverse { -tap.dx } else { tap.dx };
                    let Some(target_x) = x.checked_add_signed(dx as isize) else {
                        continue;
                    };
                    let target_y = y + tap.dy as usize;
                    if target_x >= width || target_y >= height {
                        continue;
                    }
                    if fixed_alpha.is_some_and(|(cutoff, _)| {
                        source.row(target_y as u32).expect("validated target row")
                            [target_x * 4 + 3] <= cutoff
                    }) {
                        continue;
                    }
                    let target = (target_y % ROWS) * width + target_x;
                    let weight = f64::from(tap.weight) * strength_mask;
                    for axis in 0..3 {
                        let updated =
                            (f64::from(self.work[target][axis]) + error[axis] * weight) as f32;
                        if !updated.is_finite() {
                            return Err(arithmetic(ErrorPath::DiffusionWork));
                        }
                        self.work[target][axis] = updated;
                    }
                }
            }
            if y + ROWS < height {
                self.fill_row(source, y + ROWS, policy.feedback);
            }
            progress(y as u32 + 1)?;
        }
        Ok(())
    }

    fn fill_row(&mut self, source: ImageView<'_, Rgba8>, y: usize, feedback: DiffusionFeedback) {
        let width = source.dimensions().width_usize();
        for x in 0..width {
            self.work[(y % ROWS) * width + x] =
                match self.quantizer.palette().prepare_pixel(rgba(source, x, y)) {
                    PalettePixel::Index(_) => [0.0; 3],
                    PalettePixel::Color(rgb) => match feedback {
                        DiffusionFeedback::SrgbBytes => rgb.map(f32::from),
                        DiffusionFeedback::Matching => self.quantizer.converter().coordinates(rgb),
                    },
                };
        }
    }
}

fn rgba(source: ImageView<'_, Rgba8>, x: usize, y: usize) -> [u8; 4] {
    let row = source.row(y as u32).expect("validated source row");
    let start = x * 4;
    [row[start], row[start + 1], row[start + 2], row[start + 3]]
}

fn nearest_finite(
    matcher: &PaletteMatcher,
    coordinates: [f32; 3],
) -> Result<PaletteColor, Failure> {
    matcher
        .nearest_finite(coordinates)
        .ok_or_else(|| arithmetic(ErrorPath::DiffusionDistance))
}

fn arithmetic(path: ErrorPath) -> Failure {
    Failure::new(ErrorCode::Runtime, path)
}
