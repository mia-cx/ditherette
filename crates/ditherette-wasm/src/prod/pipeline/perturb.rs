//! Bounded ownership around the literal palette-free RGBA8 field kernel.

use std::mem::size_of;

use super::processor::{dimensions, Allocator, Boundary};
use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{BayerSize, Field, PerturbPolicy, Placement},
        },
        dither::{ordered, random_noise},
        tiling::RowBand,
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
    if matches!(policy.field, Field::BlueNoise {}) {
        return Err(Failure::new(
            ErrorCode::UnsupportedOperation,
            ErrorPath::PerturbField,
        ));
    }
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
    crate::prod::dither::perturb::perturb_by_field_rows_into(
        source,
        output,
        policy.space,
        policy.strength,
        policy.placement,
        RowBand::new(0, source.dimensions().height()).expect("validated dimensions"),
        |x, y, index| match policy.field {
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
            Field::BlueNoise {} => unreachable!("unsupported fields fail before allocation"),
        },
    );
}

pub(super) fn run<B: Boundary, A: Allocator>(
    request: PerturbRequest,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
) -> Result<B::Output, Failure> {
    validate(request.perturb)?;
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    // Shared Processor bookkeeping already covers two owned Vec headers.
    let owned = overhead
        .checked_add(size_of::<PerturbRequest>() as u64)
        .and_then(|n| n.checked_add(working_capacity_bytes()))
        .ok_or_else(memory_limit)?;
    let required = owned.checked_add(len as u64 * 2).ok_or_else(memory_limit)?;
    if required > limit {
        return Err(memory_limit());
    }
    *peak = owned;
    let mut source = Vec::new();
    let mut output = Vec::new();
    allocator.reserve(&mut source, len)?;
    *peak = owned + source.capacity() as u64;
    if *peak + len as u64 > limit {
        return Err(memory_limit());
    }
    allocator.reserve(&mut output, len)?;
    *peak += output.capacity() as u64;
    if *peak > limit {
        return Err(memory_limit());
    }
    source.resize(len, 0);
    output.resize(len, 0);
    boundary.copy_input(&mut source)?;
    execute(
        ImageView::packed(&source, dimensions).expect("validated source storage"),
        ImageViewMut::packed(&mut output, dimensions).expect("reserved output storage"),
        request.perturb,
    );
    boundary.complete(&output, dimensions)
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
