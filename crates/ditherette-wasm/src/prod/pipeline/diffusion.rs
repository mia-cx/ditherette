//! Complete-call ownership around the three-row scalar diffusion kernel.

use super::{
    processor::{dimensions, Allocator},
    quantize::{preparation_failure, QuantizeBoundary, QuantizeRequest},
};
use crate::{
    image::{ImageView, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::DitherPolicy,
        },
        dither::error_diffusion::prepared::{DiffusionPolicy, PreparedDiffusion},
    },
};

pub(super) fn run<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    dither: DitherPolicy,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
) -> Result<B::Output, Failure> {
    let policy = DiffusionPolicy::new(dither)?;
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let source_len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != source_len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    let output_len = source_len / 4;
    let prepared_bytes = PreparedDiffusion::required_capacity_bytes(
        request.source_width,
        request.palette,
        request.alpha,
        request.matching,
    )
    .map_err(preparation_failure)?;
    let buffers_bytes = source_len as u64 + output_len as u64;
    let needed = overhead
        .checked_add(buffers_bytes)
        .and_then(|n| n.checked_add(prepared_bytes))
        .ok_or_else(memory_limit)?;
    if needed > limit {
        return Err(memory_limit());
    }
    let mut prepared = PreparedDiffusion::try_new(
        request.source_width,
        request.palette,
        request.alpha,
        request.matching,
        limit - overhead - buffers_bytes,
    )
    .map_err(preparation_failure)?;
    let owned = overhead + prepared.capacity_bytes();
    *peak = owned;
    let mut source = Vec::new();
    let mut indices = Vec::new();
    allocator.reserve(&mut source, source_len)?;
    *peak = owned + source.capacity() as u64;
    if *peak + output_len as u64 > limit {
        return Err(memory_limit());
    }
    allocator.reserve(&mut indices, output_len)?;
    *peak += indices.capacity() as u64;
    if *peak > limit {
        return Err(memory_limit());
    }
    source.resize(source_len, 0);
    indices.resize(output_len, 0);
    boundary.copy_input(&mut source)?;
    prepared.execute(
        ImageView::packed(&source, dimensions).expect("validated source storage"),
        &mut indices,
        policy,
    )?;
    boundary.complete(&indices, dimensions, prepared.palette())
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
