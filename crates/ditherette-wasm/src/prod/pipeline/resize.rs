//! Bounded full-call preparation and dispatch for landed resize kernels.

use std::{mem::size_of, num::NonZeroU32};

mod bands;
mod policy;
use bands::ResizeScratch;
pub(super) use policy::measured as measured_row_policy;

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{Anchor, DitherPolicy, ResizePolicy, Support},
        },
        resize::{
            common::{alignment::ResizeAnchor, allocation::CapacityBudget},
            scalar::{
                area, bicubic, bilinear, convolution, lanczos, nearest,
                trilinear::PreparedTrilinear,
            },
        },
    },
};

pub(super) enum PreparedResize {
    Identity,
    Nearest(nearest::NearestResizePlan, ResizeScratch<u8>),
    Area(area::AreaResizePlan, ResizeScratch<f32>),
    Bilinear(bilinear::BilinearResizePlan, ResizeScratch<f32>),
    Bicubic(bicubic::BicubicResizePlan, ResizeScratch<f64>),
    Lanczos(lanczos::LanczosResizePlan, ResizeScratch<f64>),
    Trilinear {
        scratch: Option<PreparedTrilinear<Rgba8>>,
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: Anchor,
    },
}

// PreparedResize's inline storage belongs to the preparation entry's reserved Vec.
const TRILINEAR_RECORD_BYTES: u64 = size_of::<PreparedTrilinear<Rgba8>>() as u64;

/// Measured gather cutoff includes half-sized nearest dimensions.
const SPARSE_NEAREST_SOURCE_RATIO: usize = 4;
// Larger separable calls benefit from retained perturb stages; Yliluoma stays conservative.
const SPARSE_NEAREST_STAGE_CACHE_SOURCE_RATIO: usize = 16;

/// Bound gathered storage to small fixed-support scalar outputs on both axes.
const SPARSE_CONVOLUTION_MIN_SHRINK: u32 = 16;

pub(super) struct SparseConvolution {
    pub dimensions: ImageDimensions,
    pub slots: usize,
    pub offset_bytes: usize,
}

pub(super) fn sparse_convolution(
    source: ImageDimensions,
    output: ImageDimensions,
    policy: ResizePolicy,
) -> Option<SparseConvolution> {
    if cfg!(feature = "threads")
        || output.width() > source.width() / SPARSE_CONVOLUTION_MIN_SHRINK
        || output.height() > source.height() / SPARSE_CONVOLUTION_MIN_SHRINK
    {
        return None;
    }
    let slots = match policy {
        ResizePolicy::Lanczos2 {
            support: Support::Fixed,
            ..
        } => 4,
        ResizePolicy::Lanczos3 {
            support: Support::Fixed,
            ..
        } => 6,
        _ => return None,
    };
    let dimensions = ImageDimensions::new(output.width() * slots, output.height() * slots).unwrap();
    Some(SparseConvolution {
        dimensions,
        slots: slots as usize,
        offset_bytes: (dimensions.width_usize() + dimensions.height_usize()) * 4,
    })
}

pub(super) fn sparse_nearest(
    source_len: usize,
    output_len: usize,
    policy: ResizePolicy,
    direct_output: bool,
) -> bool {
    matches!(policy, ResizePolicy::Nearest { .. })
        && (output_len <= source_len / SPARSE_NEAREST_SOURCE_RATIO
            || (direct_output && output_len > source_len))
}

pub(super) fn sparse_nearest_for_process(
    source_len: usize,
    output_len: usize,
    policy: ResizePolicy,
    dither: DitherPolicy,
) -> bool {
    let ratio = match dither {
        DitherPolicy::None {} | DitherPolicy::Diffusion { .. } => SPARSE_NEAREST_SOURCE_RATIO,
        DitherPolicy::Separable { .. } | DitherPolicy::Yliluoma { .. } => {
            SPARSE_NEAREST_STAGE_CACHE_SOURCE_RATIO
        }
    };
    matches!(policy, ResizePolicy::Nearest { .. }) && output_len <= source_len / ratio
}

pub(super) fn sparse_nearest_offset_bytes(output: ImageDimensions) -> usize {
    (output.width() as usize + output.height() as usize) * size_of::<u32>()
}

#[cfg(test)]
mod sparse_policy_tests {
    use super::*;
    use crate::prod::contract::request::{
        BayerSize, Diffusion, DiffusionFeedback, Field, PerturbPolicy, Placement, WorkingSpace,
    };

    #[test]
    fn sparse_convolution_requires_fixed_support_and_sixteenfold_shrink_on_both_axes() {
        let source = ImageDimensions::new(320, 240).unwrap();
        let fixed = ResizePolicy::Lanczos3 {
            anchor: Anchor::Center,
            support: Support::Fixed,
        };
        assert_eq!(
            sparse_convolution(source, ImageDimensions::new(20, 15).unwrap(), fixed).is_some(),
            !cfg!(feature = "threads")
        );
        for output in [(21, 15), (20, 16), (320, 15), (20, 240)] {
            assert!(sparse_convolution(
                source,
                ImageDimensions::new(output.0, output.1).unwrap(),
                fixed
            )
            .is_none());
        }
        assert!(sparse_convolution(
            source,
            ImageDimensions::new(16, 12).unwrap(),
            ResizePolicy::Lanczos3 {
                anchor: Anchor::Center,
                support: Support::ScaleAware
            }
        )
        .is_none());
    }

    #[test]
    fn fused_half_size_gathers_none_and_diffusion_but_keeps_cached_dither_stages() {
        let nearest = ResizePolicy::Nearest {
            anchor: Anchor::Center,
        };
        for (dither, half_size_gathers) in [
            (DitherPolicy::None {}, true),
            (
                DitherPolicy::Diffusion {
                    kernel: Diffusion::FloydSteinberg,
                    feedback: DiffusionFeedback::SrgbBytes,
                    strength: 1.0,
                    serpentine: true,
                    placement: Placement::Everywhere {},
                },
                true,
            ),
            (
                DitherPolicy::Separable {
                    perturb: PerturbPolicy {
                        field: Field::Bayer {
                            size: BayerSize::Eight,
                        },
                        space: WorkingSpace::Srgb,
                        strength: 1.0,
                        placement: Placement::Everywhere {},
                    },
                },
                false,
            ),
            (
                DitherPolicy::Yliluoma {
                    size: BayerSize::Eight,
                    placement: Placement::Everywhere {},
                },
                false,
            ),
        ] {
            assert_eq!(
                sparse_nearest_for_process(64 * 64 * 4, 32 * 32 * 4, nearest, dither),
                half_size_gathers
            );
            assert!(sparse_nearest_for_process(
                64 * 64 * 4,
                16 * 16 * 4,
                nearest,
                dither
            ));
        }
    }

    #[test]
    fn nearest_gathers_small_outputs_and_direct_upscales_only() {
        let nearest = ResizePolicy::Nearest {
            anchor: Anchor::Center,
        };
        let source_bytes = 64 * 64 * 4;
        for direct in [false, true] {
            assert!(sparse_nearest(source_bytes, 32 * 32 * 4, nearest, direct));
            assert!(!sparse_nearest(
                source_bytes,
                (32 * 32 + 1) * 4,
                nearest,
                direct
            ));
            assert!(!sparse_nearest(source_bytes, source_bytes, nearest, direct));
            assert_eq!(
                sparse_nearest(source_bytes, 96 * 96 * 4, nearest, direct),
                direct
            );
            for output_bytes in [32 * 32 * 4, 96 * 96 * 4] {
                assert!(!sparse_nearest(
                    source_bytes,
                    output_bytes,
                    ResizePolicy::Area {},
                    direct
                ));
            }
        }
    }
}

pub(super) fn supported(policy: ResizePolicy) -> Result<(), Failure> {
    match policy {
        ResizePolicy::Nearest { .. }
        | ResizePolicy::Area {}
        | ResizePolicy::Bilinear { .. }
        | ResizePolicy::Bicubic { .. }
        | ResizePolicy::Lanczos2 { .. }
        | ResizePolicy::Lanczos3 { .. }
        | ResizePolicy::Trilinear { .. } => Ok(()),
    }
}

impl PreparedResize {
    pub(super) fn write_convolution_source_offsets<'a>(
        &self,
        slots: usize,
        offsets: &'a mut [u8],
    ) -> (&'a [u8], &'a [u8]) {
        let Self::Lanczos(plan, _) = self else {
            unreachable!("validated sparse Lanczos policy")
        };
        plan.write_sparse_offsets(slots, offsets)
    }

    pub(super) fn execute_sparse_convolution(
        &mut self,
        source: ImageView<'_, Rgba8>,
        output: ImageViewMut<'_, Rgba8>,
        slots: usize,
        columns: &[u8],
        rows: &[u8],
    ) -> Result<(), Failure> {
        let Self::Lanczos(plan, _) = self else {
            unreachable!("validated sparse Lanczos policy")
        };
        plan.resize_sparse_fixed(source, output, slots, columns, rows)
    }

    pub(super) fn write_nearest_source_offsets<'a>(
        &self,
        offsets: &'a mut [u8],
    ) -> (&'a [u8], &'a [u8]) {
        let Self::Nearest(plan, _) = self else {
            unreachable!("sparse input requires a nonidentity nearest plan")
        };
        plan.write_source_offsets(offsets)
    }

    pub(super) fn required_bytes(
        source: ImageDimensions,
        output: ImageDimensions,
        policy: ResizePolicy,
    ) -> Result<u64, Failure> {
        supported(policy)?;
        if source == output {
            return Ok(0);
        }
        match policy {
            ResizePolicy::Nearest { .. } => Ok(Self::required_nearest_bytes(source, output)),
            ResizePolicy::Area {} => area::AreaResizePlan::required_bytes(source, output),
            ResizePolicy::Bilinear { anchor } => {
                bilinear::BilinearResizePlan::required_bytes(source, output, resize_anchor(anchor))
            }
            ResizePolicy::Bicubic { support, .. } => bicubic::BicubicResizePlan::required_bytes(
                source,
                output,
                convolution_support(support),
            ),
            ResizePolicy::Lanczos2 { support, .. } | ResizePolicy::Lanczos3 { support, .. } => {
                let radius = if matches!(policy, ResizePolicy::Lanczos2 { .. }) {
                    2
                } else {
                    3
                };
                lanczos::LanczosResizePlan::required_bytes(
                    source,
                    output,
                    NonZeroU32::new(radius).unwrap(),
                    convolution_support(support),
                )
            }
            ResizePolicy::Trilinear { .. } => {
                PreparedTrilinear::<Rgba8>::required_bytes(source, output)
                    .map(|bytes| bytes - TRILINEAR_RECORD_BYTES)
            }
        }
    }

    pub(super) fn new(
        source: ImageDimensions,
        output: ImageDimensions,
        policy: ResizePolicy,
        limit: u64,
    ) -> Result<Self, Failure> {
        let mut budget = CapacityBudget::new(limit);
        budget.check_additional(Self::required_bytes(source, output, policy)?)?;
        if source == output {
            return Ok(Self::Identity);
        }
        match policy {
            ResizePolicy::Nearest { anchor } => Self::new_nearest(source, output, anchor, limit),
            ResizePolicy::Area {} => {
                let plan = area::AreaResizePlan::try_new(source, output, &mut budget)?;
                let mut scratch = budget.vector(plan.scratch_elements())?;
                scratch.resize(plan.scratch_elements(), 0.0);
                Ok(Self::Area(plan, scratch.into()))
            }
            ResizePolicy::Bilinear { anchor } => {
                let plan = bilinear::BilinearResizePlan::try_new(
                    source,
                    output,
                    resize_anchor(anchor),
                    &mut budget,
                )?;
                let mut scratch = budget.vector(plan.scratch_elements())?;
                scratch.resize(plan.scratch_elements(), 0.0);
                Ok(Self::Bilinear(plan, scratch.into()))
            }
            ResizePolicy::Bicubic { anchor, support } => {
                let plan = bicubic::BicubicResizePlan::try_new(
                    source,
                    output,
                    resize_anchor(anchor),
                    convolution_support(support),
                    &mut budget,
                )?;
                let elements = plan.scratch_elements()?;
                let mut scratch = budget.vector(elements)?;
                scratch.resize(elements, 0.0);
                Ok(Self::Bicubic(plan, scratch.into()))
            }
            ResizePolicy::Lanczos2 { anchor, support }
            | ResizePolicy::Lanczos3 { anchor, support } => {
                let constructor = if matches!(policy, ResizePolicy::Lanczos2 { .. }) {
                    lanczos::LanczosResizePlan::try_new2
                } else {
                    lanczos::LanczosResizePlan::try_new3
                };
                let plan = constructor(
                    source,
                    output,
                    resize_anchor(anchor),
                    convolution_support(support),
                    &mut budget,
                )?;
                let elements = plan.scratch_elements()?;
                let mut scratch = budget.vector(elements)?;
                scratch.resize(elements, 0.0);
                Ok(Self::Lanczos(plan, scratch.into()))
            }
            ResizePolicy::Trilinear { anchor } => PreparedTrilinear::try_new(
                source,
                output,
                resize_anchor(anchor),
                limit + TRILINEAR_RECORD_BYTES,
            )
            .map(|scratch| Self::Trilinear {
                scratch: Some(scratch),
                source,
                output,
                anchor,
            }),
        }
    }

    /// The landed nearest planner's bound, without dispatching through unrelated filters.
    pub(super) fn required_nearest_bytes(source: ImageDimensions, output: ImageDimensions) -> u64 {
        nearest::NearestResizePlan::required_capacity_bytes(source, output)
    }

    /// Builds nonidentity nearest metadata with the same fallible constructor and errors.
    pub(super) fn new_nearest(
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: Anchor,
        limit: u64,
    ) -> Result<Self, Failure> {
        nearest::NearestResizePlan::try_new(source, output, resize_anchor(anchor), limit)
            .map(|plan| Self::Nearest(plan, Vec::new().into()))
            .map_err(|error| match error {
                nearest::PlanAllocationError::MemoryLimit => {
                    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
                }
                nearest::PlanAllocationError::Allocation => {
                    Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
                }
            })
    }

    pub(super) fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Identity => 0,
            Self::Nearest(plan, scratch) => plan.capacity_bytes() + scratch.capacity_bytes(),
            Self::Area(plan, scratch) => plan.capacity_bytes() + scratch.capacity_bytes(),
            Self::Bilinear(plan, scratch) => plan.capacity_bytes() + scratch.capacity_bytes(),
            Self::Bicubic(plan, scratch) => plan.capacity_bytes() + scratch.capacity_bytes(),
            Self::Lanczos(plan, scratch) => plan.capacity_bytes() + scratch.capacity_bytes(),
            Self::Trilinear { scratch, .. } => scratch
                .as_ref()
                .map_or(0, |plan| plan.capacity_bytes() - TRILINEAR_RECORD_BYTES),
        }
    }

    /// Mutable storage is outside the retained preparation cap and is evicted first.
    pub(super) fn scratch_capacity_bytes(&self) -> u64 {
        match self {
            Self::Nearest(_, scratch) => scratch.capacity_bytes(),
            Self::Area(_, scratch) | Self::Bilinear(_, scratch) => scratch.capacity_bytes(),
            Self::Bicubic(_, scratch) | Self::Lanczos(_, scratch) => scratch.capacity_bytes(),
            Self::Trilinear { .. } => self.capacity_bytes(),
            _ => 0,
        }
    }

    pub(super) fn required_scratch_bytes(&self) -> Result<u64, Failure> {
        Ok(match self {
            Self::Area(plan, _) => plan.scratch_elements() as u64 * 4,
            Self::Bilinear(plan, _) => plan.scratch_elements() as u64 * 4,
            Self::Bicubic(plan, _) => plan.scratch_elements()? as u64 * 8,
            Self::Lanczos(plan, _) => plan.scratch_elements()? as u64 * 8,
            Self::Trilinear { source, output, .. } => {
                PreparedTrilinear::<Rgba8>::required_bytes(*source, *output)?
                    - TRILINEAR_RECORD_BYTES
            }
            _ => 0,
        })
    }

    pub(super) fn drop_scratch(&mut self) {
        match self {
            Self::Nearest(_, scratch) => scratch.clear(),
            Self::Area(_, scratch) | Self::Bilinear(_, scratch) => scratch.clear(),
            Self::Bicubic(_, scratch) | Self::Lanczos(_, scratch) => scratch.clear(),
            Self::Trilinear { scratch, .. } => *scratch = None,
            _ => {}
        }
    }

    /// Rebuild released scratch with the landed fallible helpers, before source import.
    pub(super) fn restore_scratch(&mut self, limit: u64) -> Result<(), Failure> {
        let retained = self.capacity_bytes() - self.scratch_capacity_bytes();
        let mut budget =
            CapacityBudget::new(limit.checked_sub(retained).ok_or_else(|| {
                Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
            })?);
        match self {
            Self::Area(plan, scratch) => scratch.restore(plan.scratch_elements(), &mut budget)?,
            Self::Bilinear(plan, scratch) => {
                scratch.restore(plan.scratch_elements(), &mut budget)?
            }
            Self::Bicubic(plan, scratch) => {
                scratch.restore(plan.scratch_elements()?, &mut budget)?
            }
            Self::Lanczos(plan, scratch) => {
                scratch.restore(plan.scratch_elements()?, &mut budget)?
            }
            Self::Trilinear {
                scratch,
                source,
                output,
                anchor,
            } if scratch.is_none() => {
                *scratch = Some(PreparedTrilinear::try_new(
                    *source,
                    *output,
                    resize_anchor(*anchor),
                    limit + TRILINEAR_RECORD_BYTES,
                )?);
            }
            _ => {}
        }
        Ok(())
    }

    /// Shares existing plans and keeps only the selected scratch owner. Trilinear retains one mip chain.
    pub(super) fn select_bands(
        &mut self,
        output: ImageDimensions,
        policy: Option<super::execution::RowBandPolicy>,
        limit: u64,
    ) -> Result<(), Failure> {
        if matches!(self, Self::Nearest(..)) {
            return self.select_nearest_bands(output, policy, limit);
        }
        let retained = self.capacity_bytes() - self.scratch_capacity_bytes();
        let limit = limit
            .checked_sub(retained)
            .ok_or_else(|| Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes))?;
        match self {
            Self::Area(plan, scratch) => {
                scratch.select(output, policy, plan.scratch_elements(), limit, &|_| {
                    Ok(plan.scratch_elements())
                })
            }
            Self::Bilinear(plan, scratch) => {
                scratch.select(output, policy, plan.scratch_elements(), limit, &|_| {
                    Ok(plan.scratch_elements())
                })
            }
            Self::Bicubic(plan, scratch) => {
                scratch.select(output, policy, plan.scratch_elements()?, limit, &|band| {
                    plan.row_scratch_elements(band.y_start(), band.height())
                })
            }
            Self::Lanczos(plan, scratch) => {
                scratch.select(output, policy, plan.scratch_elements()?, limit, &|band| {
                    plan.row_scratch_elements(band.y_start(), band.height())
                })
            }
            _ => Ok(()),
        }
    }

    /// Retains the nearest plan and applies the resolved scalar or developer band policy.
    pub(super) fn select_nearest_bands(
        &mut self,
        output: ImageDimensions,
        policy: Option<super::execution::RowBandPolicy>,
        limit: u64,
    ) -> Result<(), Failure> {
        let Self::Nearest(plan, scratch) = self else {
            unreachable!("nearest band preparation")
        };
        let limit = limit
            .checked_sub(plan.capacity_bytes())
            .ok_or_else(|| Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes))?;
        scratch.select(output, policy, 0, limit, &|_| Ok(0))
    }

    /// Only the convolution kernels specialize on source opacity; others ignore the flag.
    pub(super) fn uses_source_opacity(&self) -> bool {
        matches!(self, Self::Bicubic(..) | Self::Lanczos(..))
    }

    pub(super) fn execute_known_opacity(
        &mut self,
        source: ImageView<'_, Rgba8>,
        output: ImageViewMut<'_, Rgba8>,
        source_opaque: bool,
    ) -> Result<(), Failure> {
        self.execute_with_progress_known_opacity(source, output, source_opaque, &mut |_, _| Ok(()))
    }

    /// Reuses each prepared kernel and reports only work that its chosen path performs.
    pub(super) fn execute_with_progress_known_opacity(
        &mut self,
        source: ImageView<'_, Rgba8>,
        mut output: ImageViewMut<'_, Rgba8>,
        source_opaque: bool,
        progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        let height = output.dimensions().height();
        match self {
            Self::Identity => {
                progress(0, height)?;
                output.data_mut().copy_from_slice(source.data());
                progress(height, height)?;
            }
            Self::Nearest(plan, scratch) => {
                if let Some(result) = scratch.execute(
                    &mut output,
                    &|band, output, _| {
                        nearest::resize_nearest_rgba8_rows_with_plan_into(
                            source,
                            output,
                            plan,
                            band.y_start(),
                        );
                        Ok(())
                    },
                    progress,
                ) {
                    return result;
                }
                // Keep its optimized repeat/copy dispatch intact; this whole call is one work batch.
                progress(0, height)?;
                nearest::resize_nearest_rgba8_with_plan_into(source, output, plan);
                progress(height, height)?;
            }
            Self::Area(plan, scratch) => {
                if let Some(result) = scratch.execute(
                    &mut output,
                    &|band, output, scratch| {
                        area::resize_area_rgba8_rows_with_plan_and_scratch_into(
                            source,
                            output,
                            plan,
                            band.y_start(),
                            scratch,
                        )
                    },
                    progress,
                ) {
                    return result;
                }
                progress(0, height)?;
                area::resize_area_with_progress(
                    source,
                    output,
                    plan,
                    scratch.scalar(),
                    &mut |rows| progress(rows, height),
                )?;
            }
            Self::Bilinear(plan, scratch) => {
                if let Some(result) = scratch.execute(
                    &mut output,
                    &|band, output, scratch| {
                        bilinear::resize_bilinear_rgba8_rows_with_plan_and_scratch_into(
                            source,
                            output,
                            plan,
                            band.y_start(),
                            scratch,
                        )
                    },
                    progress,
                ) {
                    return result;
                }
                progress(0, height)?;
                bilinear::resize_bilinear_with_progress(
                    source,
                    output,
                    plan,
                    scratch.scalar(),
                    &mut |rows| progress(rows, height),
                )?;
            }
            Self::Bicubic(plan, scratch) => {
                if let Some(result) = scratch.execute(
                    &mut output,
                    &|band, output, scratch| {
                        bicubic::resize_bicubic_rgba8_rows_with_plan_and_scratch_known_opacity_into(
                            source,
                            output,
                            plan,
                            band.y_start(),
                            scratch,
                            source_opaque,
                        )
                    },
                    progress,
                ) {
                    return result;
                }
                bicubic::resize_bicubic_with_progress_known_opacity(
                    source,
                    output,
                    plan,
                    scratch.scalar(),
                    source_opaque,
                    progress,
                )?
            }
            Self::Lanczos(plan, scratch) => {
                if let Some(result) = scratch.execute(
                    &mut output,
                    &|band, output, scratch| {
                        lanczos::resize_lanczos_rgba8_rows_with_plan_and_scratch_known_opacity_into(
                            source,
                            output,
                            plan,
                            band.y_start(),
                            scratch,
                            source_opaque,
                        )
                    },
                    progress,
                ) {
                    return result;
                }
                lanczos::resize_lanczos_with_progress_known_opacity(
                    source,
                    output,
                    plan,
                    scratch.scalar(),
                    source_opaque,
                    progress,
                )?
            }
            Self::Trilinear { scratch, .. } => scratch
                .as_mut()
                .expect("reserved trilinear scratch")
                .execute_with_progress(source, output, progress)?,
        }
        Ok(())
    }
}

fn restore_vector<T: Default + Clone>(
    scratch: &mut Vec<T>,
    length: usize,
    budget: &mut CapacityBudget,
) -> Result<(), Failure> {
    if scratch.capacity() < length {
        *scratch = budget.vector(length)?;
    } else {
        budget.check_additional((scratch.capacity() * size_of::<T>()) as u64)?;
    }
    scratch.resize(length, T::default());
    Ok(())
}

fn resize_anchor(anchor: Anchor) -> ResizeAnchor {
    use ResizeAnchor as A;
    match anchor {
        Anchor::TopLeft => A::TopLeft,
        Anchor::Top => A::Top,
        Anchor::TopRight => A::TopRight,
        Anchor::Left => A::Left,
        Anchor::Center => A::Center,
        Anchor::Right => A::Right,
        Anchor::BottomLeft => A::BottomLeft,
        Anchor::Bottom => A::Bottom,
        Anchor::BottomRight => A::BottomRight,
    }
}

fn convolution_support(support: Support) -> convolution::SupportPolicy {
    match support {
        Support::Fixed => convolution::SupportPolicy::Fixed,
        Support::ScaleAware => convolution::SupportPolicy::ScaleAware,
    }
}
