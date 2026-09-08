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
    store: &mut super::preparation::Store,
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
    PreparedDiffusion::required_capacity_bytes(
        request.source_width,
        request.palette,
        request.alpha,
        request.matching,
    )
    .map_err(preparation_failure)?;
    let mut call = super::preparation::Call::new(
        store,
        Some(request),
        None,
        [source_len, output_len, 0, 0],
        request.source_width as usize * 3,
        overhead,
        limit,
        peak,
        allocator,
    )?;
    let (prepared, _, scratch) = call.parts();
    let prepared = prepared.expect("requested palette");
    let [source, indices, _, _] = &mut scratch.buffers;
    boundary.copy_input(source)?;
    crate::prod::dither::error_diffusion::prepared::execute_with_scratch(
        prepared,
        &mut scratch.diffusion,
        ImageView::packed(source, dimensions).expect("validated source storage"),
        indices,
        policy,
    )?;
    let result = boundary.complete(indices, dimensions, prepared.palette());
    call.finish(result)
}
