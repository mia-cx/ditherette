//! Resize-then-dither composition with complete call preflight and both RGBA8 boundaries.

use super::{
    perturb,
    preparation::{Call, ResizePreparation, Store},
    processor::{dimensions, Allocator},
    quantize::{preparation_failure, QuantizeBoundary, QuantizeRequest},
    resize::PreparedResize,
};
use crate::{
    image::{contracts::PaletteEntry, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{BayerSize, DitherPolicy, RecipeV1},
        },
        dither::error_diffusion::prepared::{execute_with_scratch, DiffusionPolicy},
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
    let resized_len = output_dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidSettings, ErrorPath::Output))?;
    let indices_len = resized_len / 4;
    let perturb_len = if matches!(recipe.dither, DitherPolicy::Separable { .. }) {
        resized_len
    } else {
        0
    };
    let diffusion = match recipe.dither {
        DitherPolicy::None {} => None,
        DitherPolicy::Separable { perturb: policy } => {
            perturb::validate(policy)?;
            None
        }
        DitherPolicy::Diffusion { .. } => Some(DiffusionPolicy::new(recipe.dither)?),
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
            None
        }
    };
    PreparedQuantizer::required_capacity_bytes(request.palette, recipe.alpha, recipe.matching)
        .map_err(preparation_failure)?;
    PreparedResize::required_bytes(source_dimensions, output_dimensions, recipe.output.resize)?;
    if boundary.input_len()? != source_len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    let mut call = Call::new(
        store,
        Some(QuantizeRequest {
            source_width: recipe.output.width,
            source_height: recipe.output.height,
            palette: request.palette,
            alpha: recipe.alpha,
            matching: recipe.matching,
        }),
        Some(ResizePreparation {
            source: source_dimensions,
            output: recipe.output,
        }),
        [source_len, resized_len, perturb_len, indices_len],
        if diffusion.is_some() {
            recipe.output.width as usize * 3
        } else {
            0
        },
        overhead,
        limit,
        peak,
        allocator,
    )?;
    let (prepared, resize, scratch) = call.parts();
    let prepared = prepared.expect("requested palette");
    let [source, resized, perturbed, indices] = &mut scratch.buffers;
    boundary.copy_input(source)?;
    resize.expect("requested resize").execute(
        ImageView::packed(source, source_dimensions).expect("validated source storage"),
        ImageViewMut::packed(resized, output_dimensions).expect("reserved resized storage"),
    )?;
    let resized = ImageView::packed(resized, output_dimensions).expect("complete resized RGBA8");
    match recipe.dither {
        DitherPolicy::Diffusion { .. } => execute_with_scratch(
            prepared,
            &mut scratch.diffusion,
            resized,
            indices,
            diffusion.expect("validated diffusion"),
        )?,
        DitherPolicy::Separable { perturb: policy } => {
            perturb::execute(
                resized,
                ImageViewMut::packed(perturbed, output_dimensions)
                    .expect("reserved perturb storage"),
                policy,
            );
            prepared.quantize_into(
                ImageView::packed(perturbed, output_dimensions).expect("complete perturb RGBA8"),
                indices,
            );
        }
        DitherPolicy::Yliluoma { size, placement } => {
            use crate::prod::dither::ordered::BayerSize as Matrix;
            let matrix = match size {
                BayerSize::Two => Matrix::Two,
                BayerSize::Four => Matrix::Four,
                BayerSize::Eight => Matrix::Eight,
                BayerSize::Sixteen => Matrix::Sixteen,
            };
            crate::prod::dither::yiluoma::dither_yiluoma_into(
                resized, prepared, indices, matrix, placement,
            );
        }
        DitherPolicy::None {} => prepared.quantize_into(resized, indices),
    }
    let result = boundary.complete(indices, output_dimensions, prepared.palette());
    call.finish(result)
}
