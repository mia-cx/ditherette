//! Complete naive method compositions. Each intermediate owns its logical bytes.
//!
//! Separable dithering crosses the rounded/clipped RGBA8 boundary before alpha
//! preparation and matching. No fused arithmetic can bypass that boundary.

pub mod processor;

use super::{
    contract::{
        error::DitheretteError,
        lifecycle::{Progress, Stage},
        request::{
            DitherPolicy, DitherQuantizeRequest, PerturbRequest, ProcessRequest, QuantizeRequest,
            Request, ResizeRequest, Source,
        },
    },
    dither::{error_diffusion, perturb as field, yiluoma},
    quantize, resize,
};
use crate::image::contracts::{IndexedImage, Rgba8Image};

/// The two owned result kinds returned by the five typed operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedImage {
    Rgba8(Rgba8Image),
    Indexed(IndexedImage),
}

impl ProcessedImage {
    /// Number of completed output pixels, independent of result storage format.
    pub fn pixel_count(&self) -> u64 {
        let dimensions = match self {
            Self::Rgba8(image) => image.dimensions(),
            Self::Indexed(image) => image.indices.dimensions(),
        };
        u64::from(dimensions.width()) * u64::from(dimensions.height())
    }
}

/// Validates a palette-free request and returns durable perturbed RGBA8.
pub fn perturb(request: PerturbRequest<'_>) -> Result<Rgba8Image, DitheretteError> {
    let layout = Request::Perturb(request).validate()?;
    Ok(field::perturb(layout.source, request.perturb)
        .expect("validated output dimensions fit RGBA8 storage"))
}

/// Quantizes every dither family, materializing RGBA8 for separable fields.
pub fn dither_and_quantize(
    request: DitherQuantizeRequest<'_>,
) -> Result<IndexedImage, DitheretteError> {
    Request::DitherAndQuantize(request).validate()?;
    dither_with_progress(request, &mut |_| Ok(()))
}

/// Resizes first, then applies exactly the standalone dither-and-quantize operation.
pub fn process(request: ProcessRequest<'_>) -> Result<IndexedImage, DitheretteError> {
    Request::Process(request).validate()?;
    process_with_progress(request, &mut |_| Ok(()))
}

/// Executes any typed method without callbacks or retained state.
pub fn execute(request: Request<'_>) -> Result<ProcessedImage, DitheretteError> {
    execute_with_progress(request, &mut |_| Ok(()))
}

type Reporter<'a> = dyn FnMut(Progress) -> Result<(), DitheretteError> + 'a;

fn execute_with_progress(
    request: Request<'_>,
    report: &mut Reporter<'_>,
) -> Result<ProcessedImage, DitheretteError> {
    request.validate()?;
    report(Progress {
        stage: Stage::Prepare,
        completed: None,
        total: None,
    })?;
    match request {
        Request::Process(request) => {
            process_with_progress(request, report).map(ProcessedImage::Indexed)
        }
        Request::Resize(request) => stage(report, Stage::Resize, output_pixels(request), || {
            resize::resize(request)
        })
        .map(ProcessedImage::Rgba8),
        Request::Perturb(request) => stage(
            report,
            Stage::Perturb,
            source_pixels(request.source),
            || perturb(request),
        )
        .map(ProcessedImage::Rgba8),
        Request::Quantize(request) => stage(
            report,
            Stage::Quantize,
            source_pixels(request.source),
            || quantize::quantize(request),
        )
        .map(ProcessedImage::Indexed),
        Request::DitherAndQuantize(request) => {
            dither_with_progress(request, report).map(ProcessedImage::Indexed)
        }
    }
}

fn process_with_progress(
    request: ProcessRequest<'_>,
    report: &mut Reporter<'_>,
) -> Result<IndexedImage, DitheretteError> {
    let resize_request = ResizeRequest {
        version: request.recipe.version,
        source: request.source,
        output: request.recipe.output,
    };
    let resized = stage(report, Stage::Resize, output_pixels(resize_request), || {
        resize::resize(resize_request)
    })?;
    dither_with_progress(
        DitherQuantizeRequest {
            quantize: QuantizeRequest {
                version: request.recipe.version,
                source: image_source(&resized),
                palette: request.palette,
                alpha: request.recipe.alpha,
                matching: request.recipe.matching,
            },
            dither: request.recipe.dither,
        },
        report,
    )
}

fn dither_with_progress(
    request: DitherQuantizeRequest<'_>,
    report: &mut Reporter<'_>,
) -> Result<IndexedImage, DitheretteError> {
    let pixels = source_pixels(request.quantize.source);
    match request.dither {
        DitherPolicy::None {} => stage(report, Stage::Quantize, pixels, || {
            quantize::quantize(request.quantize)
        }),
        DitherPolicy::Separable { perturb: policy } => {
            let intermediate = stage(report, Stage::Perturb, pixels, || {
                perturb(PerturbRequest {
                    version: request.quantize.version,
                    source: request.quantize.source,
                    perturb: policy,
                })
            })?;
            stage(report, Stage::Quantize, pixels, || {
                quantize::quantize(QuantizeRequest {
                    source: image_source(&intermediate),
                    ..request.quantize
                })
            })
        }
        DitherPolicy::Diffusion { .. } => stage(report, Stage::DitherAndQuantize, pixels, || {
            error_diffusion::diffuse(request)
        }),
        DitherPolicy::Yliluoma { .. } => stage(report, Stage::DitherAndQuantize, pixels, || {
            yiluoma::dither_yiluoma(request)
        }),
    }
}

fn stage<T>(
    report: &mut Reporter<'_>,
    stage: Stage,
    total: u64,
    operation: impl FnOnce() -> Result<T, DitheretteError>,
) -> Result<T, DitheretteError> {
    report(Progress {
        stage,
        completed: Some(0),
        total: Some(total),
    })?;
    let output = operation()?;
    report(Progress {
        stage,
        completed: Some(total),
        total: Some(total),
    })?;
    Ok(output)
}

fn image_source(image: &Rgba8Image) -> Source<'_> {
    Source {
        width: image.dimensions().width(),
        height: image.dimensions().height(),
        data: image.data(),
    }
}

fn source_pixels(source: Source<'_>) -> u64 {
    u64::from(source.width) * u64::from(source.height)
}

fn output_pixels(request: ResizeRequest<'_>) -> u64 {
    u64::from(request.output.width) * u64::from(request.output.height)
}
