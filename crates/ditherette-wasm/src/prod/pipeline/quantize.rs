//! Bounded call ownership for ordinary-space direct quantization.

use super::processor::{dimensions, Allocator};
use crate::{
    image::{
        contracts::{NormalizedPalette, PaletteEntry, ProcessWarning},
        ImageDimensions, Rgba8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{AlphaPolicy, DitherPolicy, MatchPolicy, PerturbPolicy},
        },
        palette::{PreparationError, PreparedPalette},
        quantize::PreparedQuantizer,
    },
};

/// Already typed settings. The public adapter validates raw version/tags before entering Rust.
#[derive(Clone, Copy)]
pub struct QuantizeRequest<'a> {
    pub source_width: u32,
    pub source_height: u32,
    pub palette: &'a [PaletteEntry],
    pub alpha: AlphaPolicy,
    pub matching: MatchPolicy,
}

/// Borrowed result metadata independent of the prepared matching implementation.
#[derive(Clone, Copy)]
pub struct IndexedMetadataRef<'a> {
    pub palette: &'a NormalizedPalette,
    pub warnings: &'a [ProcessWarning],
}

impl<'a> From<&'a PreparedPalette> for IndexedMetadataRef<'a> {
    fn from(value: &'a PreparedPalette) -> Self {
        Self {
            palette: &value.palette,
            warnings: &value.warnings,
        }
    }
}

/// Every external read/copy/result construction is caught by the private Wasm adapter.
pub trait QuantizeBoundary {
    type Output;
    /// Additional owned adapter records beyond the shared scalar boundary bookkeeping.
    fn capacity_bytes(&self) -> u64 {
        0
    }
    fn input_len(&mut self) -> Result<usize, Failure>;
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure>;
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, Failure>;
}

pub(super) fn run<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut super::preparation::Store,
) -> Result<B::Output, Failure> {
    run_with_perturb(
        request, None, boundary, allocator, limit, overhead, peak, store,
    )
}

pub(super) fn run_with_perturb<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    perturb: Option<PerturbPolicy>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut super::preparation::Store,
) -> Result<B::Output, Failure> {
    run_with_dither(
        request,
        perturb.map_or(DitherPolicy::None {}, |perturb| DitherPolicy::Separable {
            perturb,
        }),
        boundary,
        allocator,
        limit,
        overhead,
        peak,
        store,
    )
}

pub(super) fn run_with_dither<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    dither: DitherPolicy,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut super::preparation::Store,
) -> Result<B::Output, Failure> {
    match dither {
        DitherPolicy::None {} => {}
        DitherPolicy::Separable { perturb } => {
            super::perturb::validate(perturb)?;
        }
        DitherPolicy::Yliluoma { placement, .. } => {
            super::perturb::validate_placement(placement).map_err(|error| {
                Failure::new(
                    error.code,
                    match error.path {
                        ErrorPath::PerturbRadius => ErrorPath::DitherRadius,
                        ErrorPath::PerturbThreshold => ErrorPath::DitherThreshold,
                        ErrorPath::PerturbSoftness => ErrorPath::DitherSoftness,
                        _ => ErrorPath::DitherPlacement,
                    },
                )
            })?;
        }
        _ => {
            return Err(Failure::new(
                ErrorCode::UnsupportedOperation,
                ErrorPath::Dither,
            ))
        }
    };
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let source_len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != source_len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    PreparedQuantizer::required_capacity_bytes(request.palette, request.alpha, request.matching)
        .map_err(preparation_failure)?;
    super::indexed::run(
        request, dimensions, dimensions, None, dither, boundary, allocator, limit, overhead, peak,
        store,
    )
}

pub(super) fn preparation_failure(error: PreparationError) -> Failure {
    let path = match error.path {
        "palette" => ErrorPath::Palette,
        "alpha.threshold" => ErrorPath::AlphaThreshold,
        "matching" => ErrorPath::Matching,
        "memoryLimitBytes" => ErrorPath::MemoryLimitBytes,
        "wasm" => ErrorPath::Wasm,
        _ => ErrorPath::Control,
    };
    Failure::new(error.code, path)
}
