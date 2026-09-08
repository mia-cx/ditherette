//! Complete-call ownership around the three-row scalar diffusion kernel.

use super::{
    processor::{dimensions, Allocator},
    quantize::{preparation_failure, QuantizeBoundary, QuantizeRequest},
};
use crate::{
    image::Rgba8,
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
    DiffusionPolicy::new(dither)?;
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let source_len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != source_len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    PreparedDiffusion::required_capacity_bytes(
        request.source_width,
        request.palette,
        request.alpha,
        request.matching,
    )
    .map_err(preparation_failure)?;
    super::indexed::run(
        request, dimensions, dimensions, None, dither, boundary, allocator, limit, overhead, peak,
        store,
    )
}
