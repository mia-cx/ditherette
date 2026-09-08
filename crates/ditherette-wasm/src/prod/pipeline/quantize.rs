//! Bounded call ownership for ordinary-space direct quantization.

use super::processor::{dimensions, Allocator};
use crate::{
    image::{contracts::PaletteEntry, ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{AlphaPolicy, BayerSize, DitherPolicy, MatchPolicy, PerturbPolicy},
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
        palette: &PreparedPalette,
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
    let perturb = match dither {
        DitherPolicy::None {} => None,
        DitherPolicy::Separable { perturb } => {
            super::perturb::validate(perturb)?;
            Some(perturb)
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
            None
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
    let output_len = source_len / 4;
    PreparedQuantizer::required_capacity_bytes(request.palette, request.alpha, request.matching)
        .map_err(preparation_failure)?;
    let intermediate_len = if perturb.is_some() { source_len } else { 0 };
    let mut call = super::preparation::Call::new(
        store,
        Some(request),
        None,
        [source_len, output_len, intermediate_len, 0],
        0,
        overhead,
        limit,
        peak,
        allocator,
    )?;
    let (prepared, _, scratch) = call.parts();
    let prepared = prepared.expect("requested palette");
    let [source, indices, intermediate, _] = &mut scratch.buffers;
    boundary.copy_input(source)?;
    let source = ImageView::<Rgba8>::packed(source, dimensions)
        .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
    let quantize_source = if let Some(policy) = perturb {
        super::perturb::execute(
            source,
            ImageViewMut::packed(intermediate, dimensions).expect("reserved intermediate storage"),
            policy,
        );
        ImageView::packed(intermediate, dimensions).expect("complete RGBA8 intermediate")
    } else {
        source
    };
    if let DitherPolicy::Yliluoma { size, placement } = dither {
        use crate::prod::dither::ordered::BayerSize as Matrix;
        let size = match size {
            BayerSize::Two => Matrix::Two,
            BayerSize::Four => Matrix::Four,
            BayerSize::Eight => Matrix::Eight,
            BayerSize::Sixteen => Matrix::Sixteen,
        };
        crate::prod::dither::yiluoma::dither_yiluoma_into(
            source, prepared, indices, size, placement,
        );
    } else {
        prepared.quantize_into(quantize_source, indices);
    }
    let result = boundary.complete(indices, dimensions, prepared.palette());
    call.finish(result)
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
