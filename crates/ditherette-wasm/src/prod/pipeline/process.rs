//! Readable resize-then-dither composition with complete call-owned preflight.
//!
//! Both RGBA8 boundaries follow the frozen process composition. Existing resize,
//! perturbation, quantization, diffusion, and mixing implementations do the work.

use std::mem::size_of;

use super::{
    perturb,
    processor::{dimensions, Allocator},
    quantize::{preparation_failure, QuantizeBoundary},
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
        dither::error_diffusion::prepared::{DiffusionPolicy, PreparedDiffusion},
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

enum PreparedDither {
    Quantize(PreparedQuantizer),
    Diffusion(PreparedDiffusion, DiffusionPolicy),
}

impl PreparedDither {
    // Each prepared implementation already counts its record. Count only the
    // enum's remaining inline storage, including the diffusion policy.
    fn record_extra(diffusion: bool) -> u64 {
        (size_of::<Self>()
            - if diffusion {
                size_of::<PreparedDiffusion>()
            } else {
                size_of::<PreparedQuantizer>()
            }) as u64
    }

    fn required_bytes(request: ProcessRequest<'_>) -> Result<u64, Failure> {
        let recipe = request.recipe;
        let diffusion = matches!(recipe.dither, DitherPolicy::Diffusion { .. });
        match recipe.dither {
            DitherPolicy::None {} => {}
            DitherPolicy::Separable { perturb: policy } => perturb::validate(policy)?,
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
        }
        let prepared = if diffusion {
            PreparedDiffusion::required_capacity_bytes(
                recipe.output.width,
                request.palette,
                recipe.alpha,
                recipe.matching,
            )
        } else {
            PreparedQuantizer::required_capacity_bytes(
                request.palette,
                recipe.alpha,
                recipe.matching,
            )
        }
        .map_err(preparation_failure)?;
        Ok(prepared + Self::record_extra(diffusion))
    }

    fn new(request: ProcessRequest<'_>, limit: u64) -> Result<Self, Failure> {
        let recipe = request.recipe;
        let diffusion = matches!(recipe.dither, DitherPolicy::Diffusion { .. });
        let budget = limit
            .checked_sub(Self::record_extra(diffusion))
            .ok_or_else(memory_limit)?;
        if diffusion {
            Ok(Self::Diffusion(
                PreparedDiffusion::try_new(
                    recipe.output.width,
                    request.palette,
                    recipe.alpha,
                    recipe.matching,
                    budget,
                )
                .map_err(preparation_failure)?,
                DiffusionPolicy::new(recipe.dither)?,
            ))
        } else {
            Ok(Self::Quantize(
                PreparedQuantizer::try_new(request.palette, recipe.alpha, recipe.matching, budget)
                    .map_err(preparation_failure)?,
            ))
        }
    }

    fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Quantize(prepared) => prepared.capacity_bytes() + Self::record_extra(false),
            Self::Diffusion(prepared, _) => prepared.capacity_bytes() + Self::record_extra(true),
        }
    }
}

pub(super) fn run<B: QuantizeBoundary, A: Allocator>(
    request: ProcessRequest<'_>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
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
    let dither_bytes = PreparedDither::required_bytes(request)?;
    let resize_bytes =
        PreparedResize::required_bytes(source_dimensions, output_dimensions, recipe.output.resize)?;
    if boundary.input_len()? != source_len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    let buffers_bytes =
        source_len as u64 + resized_len as u64 + perturb_len as u64 + indices_len as u64;
    let planned = overhead
        .checked_add(buffers_bytes)
        .and_then(|n| n.checked_add(resize_bytes))
        .and_then(|n| n.checked_add(dither_bytes))
        .ok_or_else(memory_limit)?;
    if planned > limit {
        return Err(memory_limit());
    }

    let mut resize = PreparedResize::new(
        source_dimensions,
        output_dimensions,
        recipe.output.resize,
        limit - overhead - buffers_bytes - dither_bytes,
    )?;
    *peak = overhead + resize.capacity_bytes();
    let mut dither = PreparedDither::new(request, limit - *peak - buffers_bytes)?;
    *peak += dither.capacity_bytes();
    if *peak + buffers_bytes > limit {
        return Err(memory_limit());
    }

    let mut source = Vec::new();
    let mut resized = Vec::new();
    let mut perturbed = Vec::new();
    let mut indices = Vec::new();
    let mut remaining = buffers_bytes;
    for (buffer, length) in [
        (&mut source, source_len),
        (&mut resized, resized_len),
        (&mut perturbed, perturb_len),
        (&mut indices, indices_len),
    ] {
        if length == 0 {
            continue;
        }
        allocator.reserve(buffer, length)?;
        *peak += buffer.capacity() as u64;
        remaining -= length as u64;
        if *peak + remaining > limit {
            return Err(memory_limit());
        }
    }
    source.resize(source_len, 0);
    resized.resize(resized_len, 0);
    perturbed.resize(perturb_len, 0);
    indices.resize(indices_len, 0);
    boundary.copy_input(&mut source)?;

    // Keep the complete resized RGBA8 image before entering any dither family.
    resize.execute(
        ImageView::packed(&source, source_dimensions).expect("validated source storage"),
        ImageViewMut::packed(&mut resized, output_dimensions).expect("reserved resized storage"),
    )?;
    let resized = ImageView::packed(&resized, output_dimensions).expect("complete resized RGBA8");
    match &mut dither {
        PreparedDither::Diffusion(prepared, policy) => {
            prepared.execute(resized, &mut indices, *policy)?;
            boundary.complete(&indices, output_dimensions, prepared.palette())
        }
        PreparedDither::Quantize(prepared) => {
            match recipe.dither {
                DitherPolicy::Separable { perturb: policy } => {
                    perturb::execute(
                        resized,
                        ImageViewMut::packed(&mut perturbed, output_dimensions)
                            .expect("reserved perturb storage"),
                        policy,
                    );
                    prepared.quantize_into(
                        ImageView::packed(&perturbed, output_dimensions)
                            .expect("complete perturb RGBA8"),
                        &mut indices,
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
                        resized,
                        prepared,
                        &mut indices,
                        matrix,
                        placement,
                    );
                }
                DitherPolicy::None {} => prepared.quantize_into(resized, &mut indices),
                DitherPolicy::Diffusion { .. } => {
                    unreachable!("diffusion owns its prepared variant")
                }
            }
            boundary.complete(&indices, output_dimensions, prepared.palette())
        }
    }
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
