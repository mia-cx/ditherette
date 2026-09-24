//! Bounded ownership around the literal palette-free RGBA8 field kernel.

use std::mem::size_of;

use super::processor::{dimensions, Allocator, Boundary};
use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::Stage,
            request::{BayerSize, Field, PerturbPolicy, Placement, WorkingSpace},
        },
        dither::{blue_noise, ordered, placement::AdaptivePlacementWork, random_noise},
        resize::common::allocation::CapacityBudget,
        tiling::{RowBand, RowBandBuffers},
    },
};

/// Typed native call. Validation still checks finite controls and supported field implementations.
#[derive(Clone, Copy)]
pub struct PerturbRequest {
    pub source_width: u32,
    pub source_height: u32,
    pub perturb: PerturbPolicy,
}

/// The literal forward adapter owns one temporary converter record at a time; its byte tables are shared statics.
pub(super) const fn working_capacity_bytes() -> u64 {
    size_of::<crate::prod::color::packed::Converter>() as u64
}

/// Optional call-local Bayer byte transforms; budget or allocation misses keep the scalar loop.
pub(super) struct BayerBytes {
    cells: Vec<u8>,
    width: usize,
}

impl BayerBytes {
    pub(super) fn try_new(
        dimensions: ImageDimensions,
        policy: PerturbPolicy,
        available: u64,
    ) -> Option<Self> {
        // Callers skip byte tables whenever row bands run, so threaded builds reach this
        // only on the same scalar path as scalar builds.
        if policy.space != WorkingSpace::Srgb
            || !matches!(policy.placement, Placement::Everywhere {})
            || policy.strength == 0.0
        {
            return None;
        }
        let Field::Bayer { size } = policy.field else {
            return None;
        };
        let width = match size {
            BayerSize::Two => 2,
            BayerSize::Four => 4,
            BayerSize::Eight => 8,
            BayerSize::Sixteen => 16,
        };
        let entries = width * width * 256;
        // Require at least three channel lookups per prepared table entry.
        if u64::from(dimensions.width()) * u64::from(dimensions.height()) < entries as u64 {
            return None;
        }
        let heap = available.checked_sub(size_of::<Self>() as u64)?;
        let mut cells = CapacityBudget::new(heap).vector::<u8>(entries).ok()?;
        let coordinates = crate::prod::color::srgb::byte_coordinates();
        for cell in 0..width * width {
            let threshold = field_value(
                policy.field,
                (cell % width) as u32,
                (cell / width) as u32,
                0,
            );
            let amount = f64::from(threshold) * f64::from(policy.strength) * 0.25;
            for (byte, &coordinate) in coordinates.iter().enumerate() {
                cells.push(if amount == 0.0 {
                    byte as u8
                } else {
                    let perturbed = f64::from(coordinate) + amount;
                    (perturbed.clamp(0.0, 1.0) * 255.0).round() as u8
                });
            }
        }
        Some(Self { cells, width })
    }

    pub(super) fn capacity_bytes(&self) -> u64 {
        size_of::<Self>() as u64 + self.cells.capacity() as u64
    }

    fn execute(
        &self,
        source: ImageView<'_, Rgba8>,
        mut output: ImageViewMut<'_, Rgba8>,
        mut progress: impl FnMut(u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        assert_eq!(source.dimensions(), output.dimensions());
        let mask = self.width - 1;
        for y in 0..source.dimensions().height() {
            let source = source.row(y).expect("validated source row");
            let output = output.row_mut(y).expect("validated output row");
            for (x, (pixel, target)) in source
                .chunks_exact(4)
                .zip(output.chunks_exact_mut(4))
                .enumerate()
            {
                let cell = (y as usize & mask) * self.width + (x & mask);
                let table = &self.cells[cell * 256..(cell + 1) * 256];
                target[0] = table[pixel[0] as usize];
                target[1] = table[pixel[1] as usize];
                target[2] = table[pixel[2] as usize];
                target[3] = pixel[3];
            }
            progress(y + 1)?;
        }
        Ok(())
    }
}

pub(super) fn validate(policy: PerturbPolicy) -> Result<(), Failure> {
    nonnegative(policy.strength, ErrorPath::PerturbStrength)?;
    validate_placement(policy.placement)
}

pub(super) fn validate_placement(placement: Placement) -> Result<(), Failure> {
    if let Placement::Adaptive {
        radius,
        threshold,
        softness,
    } = placement
    {
        if !(1..=32768).contains(&radius) {
            return Err(Failure::new(
                ErrorCode::InvalidSettings,
                ErrorPath::PerturbRadius,
            ));
        }
        nonnegative(threshold, ErrorPath::PerturbThreshold)?;
        nonnegative(softness, ErrorPath::PerturbSoftness)?;
    }
    Ok(())
}

fn nonnegative(value: f32, path: ErrorPath) -> Result<(), Failure> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(Failure::new(ErrorCode::InvalidSettings, path))
    }
}

/// Optional row storage belongs to the enclosing call's capacity ledger.
pub(super) fn execute_with_scratch(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    policy: PerturbPolicy,
    scratch: &mut [[f32; 3]],
    bayer: Option<&BayerBytes>,
    progress: impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    if let Some(bayer) = bayer {
        return bayer.execute(source, output, progress);
    }
    crate::prod::dither::perturb::perturb_by_field_with_scratch(
        source,
        output,
        policy.space,
        policy.strength,
        policy.placement,
        RowBand::new(0, source.dimensions().height()).expect("validated dimensions"),
        scratch,
        |x, y, index| field_value(policy.field, x, y, index),
        progress,
    )
}

/// Runs the same field recipe in preflighted bands; progress stays on the caller.
pub(super) fn execute_bands(
    source: ImageView<'_, Rgba8>,
    output: &mut [u8],
    policy: PerturbPolicy,
    work: &mut RowBandBuffers<[f32; 3]>,
    progress: &mut impl FnMut(u64) -> Result<(), Failure>,
) -> Result<(), Failure> {
    crate::prod::dither::perturb::perturb_by_field_bands_into(
        source,
        output,
        policy.space,
        policy.strength,
        policy.placement,
        work,
        |x, y, index| field_value(policy.field, x, y, index),
        progress,
    )
}

fn field_value(field: Field, x: u32, y: u32, index: u64) -> f32 {
    match field {
        Field::Bayer { size } => ordered::bayer_noise_at(
            x,
            y,
            match size {
                BayerSize::Two => ordered::BayerSize::Two,
                BayerSize::Four => ordered::BayerSize::Four,
                BayerSize::Eight => ordered::BayerSize::Eight,
                BayerSize::Sixteen => ordered::BayerSize::Sixteen,
            },
        ),
        Field::Random { seed } => random_noise::random_noise_at(seed, index),
        Field::BlueNoise {} => blue_noise::blue_noise_at(x, y),
    }
}

pub(super) fn run<B: Boundary, A: Allocator>(
    request: PerturbRequest,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut super::preparation::Store,
) -> Result<B::Output, Failure> {
    validate(request.perturb)?;
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let row_policy = store.row_policy(
        super::execution::ExecutionStage::Indexed,
        super::row_fields::measured_field(
            dimensions,
            request.perturb,
            super::execution::worker_budget(),
        ),
    );
    let len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    let enabled = boundary.progress().is_some();
    let mut progress = super::progress::Control::new(enabled);
    progress.report(boundary.progress(), Stage::Prepare, 0, 1)?;
    // Shared Processor bookkeeping already covers two owned Vec headers.
    let owned = overhead
        .checked_add(size_of::<PerturbRequest>() as u64)
        .and_then(|n| n.checked_add(working_capacity_bytes()))
        .ok_or_else(memory_limit)?;
    let mut call = super::preparation::Call::snapshot(store, len, owned, limit, peak, allocator)?;
    let parent = call.source(dimensions, |bytes, compare| {
        boundary.snapshot_input(bytes, compare)
    })?;
    let key = super::identity::stage(
        Some(parent),
        crate::prod::contract::cache::StageOptions::Perturb {
            perturb: request.perturb,
        },
    )?;
    if call.take_image(1, key) {
        let image = call.image(1).unwrap();
        let result = boundary.complete(&image.bytes, image.dimensions);
        return call.finish(progress.finish(result, boundary.progress()));
    }
    let band_plan = row_policy
        .map(|policy| super::row_fields::Plan::new(dimensions, policy, true))
        .transpose()?;
    let band_capacity = band_plan
        .as_ref()
        .map_or(0, |plan| plan.additional_capacity());
    call.charge_working_capacity(band_capacity, peak)?;
    call.prepare(None, None, [len, len, 0, 0], 0, peak, allocator)?;
    let bands = band_plan
        .as_ref()
        .map(|plan| plan.allocate(request.perturb.placement, call.available_working_capacity()))
        .transpose()?;
    let band_rows_capacity = bands.as_ref().map_or(0, |(_, rows)| *rows);
    call.charge_optional_capacity(band_rows_capacity, peak)?;
    let mut bands = bands.map(|(buffers, _)| buffers);
    let mut placement = if bands.is_none() {
        AdaptivePlacementWork::try_new(
            dimensions.width(),
            request.perturb.placement,
            call.available_working_capacity(),
        )
    } else {
        None
    };
    let placement_capacity = placement
        .as_ref()
        .map_or(0, AdaptivePlacementWork::capacity_bytes);
    call.charge_optional_capacity(placement_capacity, peak)?;
    let bayer = if bands.is_none() {
        BayerBytes::try_new(
            dimensions,
            request.perturb,
            call.available_working_capacity(),
        )
    } else {
        None
    };
    let bayer_capacity = bayer.as_ref().map_or(0, BayerBytes::capacity_bytes);
    call.charge_optional_capacity(bayer_capacity, peak)?;
    let [source, output, _, _] = &mut call.scratch.buffers;
    let source = ImageView::packed(source, dimensions).expect("validated source storage");
    if bands.is_some() || enabled {
        progress.report(
            boundary.progress(),
            Stage::Perturb,
            0,
            u64::from(dimensions.height()),
        )?;
        let mut report = |completed| {
            progress.report(
                boundary.progress(),
                Stage::Perturb,
                completed,
                u64::from(dimensions.height()),
            )
        };
        if let Some(work) = &mut bands {
            execute_bands(source, output, request.perturb, work, &mut report)?;
        } else {
            let output = ImageViewMut::packed(output, dimensions).expect("reserved output storage");
            execute_with_scratch(
                source,
                output,
                request.perturb,
                placement
                    .as_mut()
                    .map_or(&mut [], AdaptivePlacementWork::scratch),
                bayer.as_ref(),
                |completed| report(u64::from(completed)),
            )?;
        }
    } else {
        let output = ImageViewMut::packed(output, dimensions).expect("reserved output storage");
        execute_with_scratch(
            source,
            output,
            request.perturb,
            placement
                .as_mut()
                .map_or(&mut [], AdaptivePlacementWork::scratch),
            bayer.as_ref(),
            |_| Ok(()),
        )?;
    }
    drop(bayer);
    call.release_working_capacity(bayer_capacity);
    drop(placement);
    call.release_working_capacity(placement_capacity);
    drop(bands);
    call.release_working_capacity(band_capacity + band_rows_capacity);
    call.retain_rgba(1, key, 1, dimensions, peak);
    let bytes = call
        .image(1)
        .map_or(call.scratch.buffers[1].as_slice(), |image| &image.bytes);
    let result = boundary.complete(bytes, dimensions);
    call.finish(progress.finish(result, boundary.progress()))
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::RowStride;

    const SIZES: [(BayerSize, u32); 4] = [
        (BayerSize::Two, 2),
        (BayerSize::Four, 4),
        (BayerSize::Eight, 8),
        (BayerSize::Sixteen, 16),
    ];

    fn policy(size: BayerSize, strength: f32) -> PerturbPolicy {
        PerturbPolicy {
            field: Field::Bayer { size },
            space: WorkingSpace::Srgb,
            strength,
            placement: Placement::Everywhere {},
        }
    }

    #[test]
    fn bayer_tables_are_optional_and_charge_actual_capacity() {
        for (size, width) in SIZES {
            let cells = width * width;
            let dimensions = ImageDimensions::new(256, cells).unwrap();
            let bytes = size_of::<BayerBytes>() as u64 + u64::from(cells) * 256;
            let settings = policy(size, 0.7);
            let work = BayerBytes::try_new(dimensions, settings, bytes).unwrap();
            assert_eq!(work.capacity_bytes(), bytes);
            assert!(BayerBytes::try_new(dimensions, settings, bytes - 1).is_none());
            assert!(BayerBytes::try_new(dimensions, settings, 0).is_none());
            assert!(BayerBytes::try_new(dimensions, policy(size, 0.0), bytes).is_none());
            assert!(BayerBytes::try_new(
                ImageDimensions::new(255, cells).unwrap(),
                settings,
                bytes
            )
            .is_none());
            for excluded in [
                PerturbPolicy {
                    field: Field::Random { seed: 0 },
                    ..settings
                },
                PerturbPolicy {
                    field: Field::BlueNoise {},
                    ..settings
                },
                PerturbPolicy {
                    space: WorkingSpace::Oklab,
                    ..settings
                },
                PerturbPolicy {
                    placement: Placement::Adaptive {
                        radius: 1,
                        threshold: 0.0,
                        softness: 1.0,
                    },
                    ..settings
                },
            ] {
                assert!(BayerBytes::try_new(dimensions, excluded, bytes).is_none());
            }
        }
    }

    #[test]
    fn bayer_tables_preserve_generic_bytes_strides_alpha_and_row_errors() {
        for (size, matrix_width) in SIZES {
            // Every Bayer cell visits every source byte; odd extents also cross tile boundaries.
            let width = matrix_width * 256 + 1;
            let height = matrix_width + 1;
            let dimensions = ImageDimensions::new(width, height).unwrap();
            let stride = RowStride::new(width as usize * 4 + 7).unwrap();
            let mut bytes = vec![203; stride.elements() * height as usize];
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let byte = (x / matrix_width as usize) as u8;
                    bytes[y * stride.elements() + x * 4..y * stride.elements() + x * 4 + 4]
                        .copy_from_slice(&[byte, byte.wrapping_mul(73), 255 - byte, byte]);
                }
            }
            let source = ImageView::new(&bytes, dimensions, stride).unwrap();
            let mut actual = vec![203; bytes.len()];
            let mut expected = actual.clone();
            for strength in [f32::from_bits(1), 0.7, 1.0, 2.0, 2.0 / 255.0, f32::MAX] {
                let settings = policy(size, strength);
                let work = BayerBytes::try_new(dimensions, settings, u64::MAX).unwrap();
                let mut rows = Vec::new();
                work.execute(
                    source,
                    ImageViewMut::new(&mut actual, dimensions, stride).unwrap(),
                    |row| {
                        rows.push(row);
                        Ok(())
                    },
                )
                .unwrap();
                crate::prod::dither::perturb::perturb_by_field_rows_into(
                    source,
                    ImageViewMut::new(&mut expected, dimensions, stride).unwrap(),
                    WorkingSpace::Srgb,
                    strength,
                    Placement::Everywhere {},
                    RowBand::new(0, height).unwrap(),
                    |x, y, index| field_value(settings.field, x, y, index),
                );
                assert_eq!(actual, expected, "{size:?}, {strength}");
                assert_eq!(rows, (1..=height).collect::<Vec<_>>());
            }
            actual.fill(203);
            let failure = Failure::new(ErrorCode::Callback, ErrorPath::Output);
            let work = BayerBytes::try_new(dimensions, policy(size, 0.7), u64::MAX).unwrap();
            assert_eq!(
                work.execute(
                    source,
                    ImageViewMut::new(&mut actual, dimensions, stride).unwrap(),
                    |_| Err(failure)
                ),
                Err(failure)
            );
            assert!(actual[stride.elements()..]
                .iter()
                .all(|&value| value == 203));
        }
    }
}
