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
) -> Result<B::Output, Failure> {
    run_with_perturb(request, None, boundary, allocator, limit, overhead, peak)
}

pub(super) fn run_with_perturb<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    perturb: Option<PerturbPolicy>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
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
    let prepared_bytes = PreparedQuantizer::required_capacity_bytes(
        request.palette,
        request.alpha,
        request.matching,
    )
    .map_err(preparation_failure)?;
    let intermediate_len = if perturb.is_some() { source_len } else { 0 };
    let buffers_bytes = source_len as u64 + output_len as u64 + intermediate_len as u64;
    let needed = overhead
        .checked_add(buffers_bytes)
        .and_then(|n| n.checked_add(prepared_bytes))
        .ok_or_else(memory_limit)?;
    if needed > limit {
        return Err(memory_limit());
    }
    let prepared = PreparedQuantizer::try_new(
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
    if *peak + output_len as u64 + intermediate_len as u64 > limit {
        return Err(memory_limit());
    }
    allocator.reserve(&mut indices, output_len)?;
    *peak += indices.capacity() as u64;
    if *peak + intermediate_len as u64 > limit {
        return Err(memory_limit());
    }
    let mut intermediate = if perturb.is_some() {
        let mut buffer = Vec::new();
        allocator.reserve(&mut buffer, intermediate_len)?;
        *peak += buffer.capacity() as u64;
        if *peak > limit {
            return Err(memory_limit());
        }
        buffer.resize(intermediate_len, 0);
        Some(buffer)
    } else {
        None
    };
    source.resize(source_len, 0);
    indices.resize(output_len, 0);
    boundary.copy_input(&mut source)?;
    let source = ImageView::<Rgba8>::packed(&source, dimensions)
        .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
    let quantize_source = if let (Some(policy), Some(buffer)) = (perturb, intermediate.as_mut()) {
        super::perturb::execute(
            source,
            ImageViewMut::packed(buffer, dimensions).expect("reserved intermediate storage"),
            policy,
        );
        ImageView::packed(buffer, dimensions).expect("complete RGBA8 intermediate")
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
            source,
            &prepared,
            &mut indices,
            size,
            placement,
        );
    } else {
        prepared.quantize_into(quantize_source, &mut indices);
    }
    boundary.complete(&indices, dimensions, prepared.palette())
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
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
