//! Resize-then-dither composition with complete call preflight and both RGBA8 boundaries.

use super::{
    perturb,
    preparation::Store,
    processor::{dimensions, Allocator},
    quantize::{preparation_failure, QuantizeBoundary, QuantizeRequest},
    resize::PreparedResize,
};
use crate::{
    image::{contracts::PaletteEntry, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{DitherPolicy, RecipeV1},
        },
        dither::error_diffusion::prepared::DiffusionPolicy,
        quantize::PreparedQuantizer,
    },
};

/// Typed native settings with a separately borrowed input/result boundary.
#[derive(Clone, Copy)]
pub struct ProcessRequest<'a> {
    pub source_width: u32,
    pub source_height: u32,
    pub palette: &'a [PaletteEntry],
    pub recipe: RecipeV1,
}

pub(super) fn run<B: QuantizeBoundary, A: Allocator>(
    request: ProcessRequest<'_>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut Store,
) -> Result<B::Output, Failure> {
    if request.recipe.version != 1 {
        return Err(Failure::new(
            ErrorCode::InvalidRequest,
            ErrorPath::RecipeVersion,
        ));
    }
    let recipe = request.recipe;
    let source_dimensions = dimensions(request.source_width, request.source_height, true)?;
    let output_dimensions = dimensions(recipe.output.width, recipe.output.height, false)?;
    let source_len = source_dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    output_dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidSettings, ErrorPath::Output))?;
    match recipe.dither {
        DitherPolicy::None {} => {}
        DitherPolicy::Separable { perturb: policy } => {
            perturb::validate(policy)?;
        }
        DitherPolicy::Diffusion { .. } => {
            DiffusionPolicy::new(recipe.dither)?;
        }
        DitherPolicy::Yliluoma { placement, .. } => {
            perturb::validate_placement(placement).map_err(|failure| {
                Failure::new(
                    failure.code,
                    match failure.path {
                        ErrorPath::PerturbRadius => ErrorPath::DitherRadius,
                        ErrorPath::PerturbThreshold => ErrorPath::DitherThreshold,
                        ErrorPath::PerturbSoftness => ErrorPath::DitherSoftness,
                        _ => ErrorPath::DitherPlacement,
                    },
                )
            })?;
        }
    };
    PreparedQuantizer::required_capacity_bytes(request.palette, recipe.alpha, recipe.matching)
        .map_err(preparation_failure)?;
    PreparedResize::required_bytes(source_dimensions, output_dimensions, recipe.output.resize)?;
    if boundary.input_len()? != source_len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    super::indexed::run(
        QuantizeRequest {
            source_width: recipe.output.width,
            source_height: recipe.output.height,
            palette: request.palette,
            alpha: recipe.alpha,
            matching: recipe.matching,
        },
        source_dimensions,
        output_dimensions,
        Some(recipe.output),
        recipe.dither,
        boundary,
        allocator,
        limit,
        overhead,
        peak,
        store,
    )
}
