//! Bounded ownership around the literal palette-free RGBA8 field kernel.

use std::mem::size_of;

use super::processor::{dimensions, Allocator, Boundary};
use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::Stage,
            request::{BayerSize, Field, PerturbPolicy, Placement},
        },
        dither::{blue_noise, ordered, random_noise},
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

/// The literal forward adapter owns one temporary converter at a time, including its byte tables.
pub(super) const fn working_capacity_bytes() -> u64 {
    size_of::<crate::prod::color::packed::Converter>() as u64
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

/// No allocation, alternate reconstruction, or source mutation occurs during field execution.
pub(super) fn execute(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    policy: PerturbPolicy,
) {
    execute_with_progress(source, output, policy, |_| Ok(()))
        .expect("disabled progress cannot fail");
}

pub(super) fn execute_with_progress(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    policy: PerturbPolicy,
    progress: impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    crate::prod::dither::perturb::perturb_by_field_with_progress(
        source,
        output,
        policy.space,
        policy.strength,
        policy.placement,
        RowBand::new(0, source.dimensions().height()).expect("validated dimensions"),
        |x, y, index| field_value(policy.field, x, y, index),
        progress,
    )
}

/// Runs the same field recipe in preflighted bands; progress stays on the caller.
pub(super) fn execute_bands(
    source: ImageView<'_, Rgba8>,
    output: &mut [u8],
    policy: PerturbPolicy,
    work: &mut RowBandBuffers<()>,
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
    boundary.copy_input(&mut call.scratch.buffers[0])?;
    let parent = super::preparation::source_key(&call.scratch.buffers[0], dimensions);
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
    let mut bands = band_plan.as_ref().map(|plan| plan.allocate()).transpose()?;
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
            execute_with_progress(source, output, request.perturb, |completed| {
                report(u64::from(completed))
            })?;
        }
    } else {
        let output = ImageViewMut::packed(output, dimensions).expect("reserved output storage");
        execute(source, output, request.perturb);
    }
    drop(bands);
    call.release_working_capacity(band_capacity);
    let content = call.content(1, 1, dimensions);
    call.retain_rgba(1, key, 1, dimensions, content, peak);
    let bytes = call
        .image(1)
        .map_or(call.scratch.buffers[1].as_slice(), |image| &image.bytes);
    let result = boundary.complete(bytes, dimensions);
    call.finish(progress.finish(result, boundary.progress()))
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
