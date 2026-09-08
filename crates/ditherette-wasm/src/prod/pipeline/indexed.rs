//! Shared materialized resize/perturb/indexed stages around the existing kernels.

use super::{
    identity, perturb,
    preparation::{source_key, Call, ResizePreparation, Store},
    processor::Allocator,
    quantize::{QuantizeBoundary, QuantizeRequest},
};
use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            cache::{Identity, StageOptions},
            failure::Failure,
            request::{BayerSize, DitherPolicy, Output},
        },
        dither::error_diffusion::prepared::{execute_with_scratch, DiffusionPolicy},
    },
};

/// Settings and storage have already passed their method-specific validation.
pub(super) fn run<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    resize: Option<Output>,
    dither: DitherPolicy,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut Store,
) -> Result<B::Output, Failure> {
    let source_len = source_dimensions
        .storage_len::<Rgba8>()
        .expect("validated source");
    let rgba_len = output_dimensions
        .storage_len::<Rgba8>()
        .expect("validated output");
    let mut call = Call::snapshot(store, source_len, overhead, limit, peak, allocator)?;
    boundary.copy_input(&mut call.scratch.buffers[0])?;
    let source = source_key(&call.scratch.buffers[0], source_dimensions);
    let palette = identity::palette_content(request.palette)?;
    let resize_key = resize
        .map(|output| identity::stage(Some(source), StageOptions::Resize { output }))
        .transpose()?;
    let resized_hit = resize_key.is_some_and(|key| call.take_image(0, key));
    let mut rgba_content = if resize.is_none() {
        Some(source)
    } else if resized_hit {
        Some(call.content(0, 1, output_dimensions))
    } else {
        None
    };
    let policy = match dither {
        DitherPolicy::Separable { perturb } => Some(perturb),
        _ => None,
    };
    let mut perturb_key = match (rgba_content, policy) {
        (Some(parent), Some(perturb)) => Some(identity::stage(
            Some(parent),
            StageOptions::Perturb { perturb },
        )?),
        _ => None,
    };
    let mut perturbed_hit = perturb_key.is_some_and(|key| call.take_image(1, key));
    let mut match_content = if policy.is_none() {
        rgba_content
    } else if perturbed_hit {
        Some(call.content(1, 2, output_dimensions))
    } else {
        None
    };
    let indexed_dither = if policy.is_some() {
        DitherPolicy::None {}
    } else {
        dither
    };
    let indexed_key = |parent: Identity| {
        identity::indexed(
            parent,
            palette,
            request.alpha,
            request.matching,
            indexed_dither,
        )
    };
    let mut final_key = match_content.map(indexed_key).transpose()?;
    if final_key.is_some_and(|key| call.take_image(2, key)) {
        let (bytes, metadata) = call.indexed_result(3);
        let result = boundary.complete(bytes, output_dimensions, metadata);
        return call.finish(result);
    }
    call.prepare(
        Some(request),
        resize
            .filter(|_| !resized_hit)
            .map(|output| ResizePreparation {
                source: source_dimensions,
                output,
            }),
        [
            source_len,
            if resize.is_some() && !resized_hit {
                rgba_len
            } else {
                0
            },
            if policy.is_some() && !perturbed_hit {
                rgba_len
            } else {
                0
            },
            rgba_len / 4,
        ],
        if matches!(dither, DitherPolicy::Diffusion { .. }) {
            output_dimensions.width() as usize * 3
        } else {
            0
        },
        peak,
        allocator,
    )?;
    if resize.is_some() && !resized_hit {
        let (_, prepared, scratch) = call.parts();
        let [source, resized, _, _] = &mut scratch.buffers;
        prepared.expect("requested resize").execute(
            ImageView::packed(source, source_dimensions).expect("owned source"),
            ImageViewMut::packed(resized, output_dimensions).expect("reserved resize"),
        )?;
        rgba_content = Some(call.content(0, 1, output_dimensions));
    }
    let rgba_content = rgba_content.expect("resized or original content");
    if let Some(perturb) = policy {
        if perturb_key.is_none() {
            let key = identity::stage(Some(rgba_content), StageOptions::Perturb { perturb })?;
            perturb_key = Some(key);
            perturbed_hit = call.take_image(1, key);
        }
        if !perturbed_hit {
            let (_, _, images, scratch) = call.image_parts();
            let [source, resized, perturbed, _] = &mut scratch.buffers;
            let rgba = images[0].map_or_else(
                || {
                    if resize.is_some() {
                        resized.as_slice()
                    } else {
                        source.as_slice()
                    }
                },
                |image| image.bytes.as_slice(),
            );
            perturb::execute(
                ImageView::packed(rgba, output_dimensions).expect("complete RGBA"),
                ImageViewMut::packed(perturbed, output_dimensions).expect("reserved perturb"),
                perturb,
            );
        }
        match_content = Some(call.content(1, 2, output_dimensions));
    } else {
        match_content = Some(rgba_content);
    }
    if final_key.is_none() {
        let key = indexed_key(match_content.unwrap())?;
        final_key = Some(key);
        call.take_image(2, key);
    }
    if call.image(2).is_none() {
        let (prepared, _, images, scratch) = call.image_parts();
        let prepared = prepared.expect("requested palette");
        let [source, resized, perturbed, indices] = &mut scratch.buffers;
        let rgba = if policy.is_some() {
            images[1].map_or(perturbed.as_slice(), |image| &image.bytes)
        } else if resize.is_some() {
            images[0].map_or(resized.as_slice(), |image| &image.bytes)
        } else {
            source.as_slice()
        };
        let view = ImageView::packed(rgba, output_dimensions).expect("complete RGBA");
        match dither {
            DitherPolicy::Diffusion { .. } => execute_with_scratch(
                prepared,
                &mut scratch.diffusion,
                view,
                indices,
                DiffusionPolicy::new(dither)?,
            )?,
            DitherPolicy::Yliluoma { size, placement } => {
                use crate::prod::dither::ordered::BayerSize as Matrix;
                let matrix = match size {
                    BayerSize::Two => Matrix::Two,
                    BayerSize::Four => Matrix::Four,
                    BayerSize::Eight => Matrix::Eight,
                    BayerSize::Sixteen => Matrix::Sixteen,
                };
                crate::prod::dither::yiluoma::dither_yiluoma_into(
                    view, prepared, indices, matrix, placement,
                );
            }
            _ => prepared.quantize_into(view, indices),
        }
    }
    if let Some(key) = resize_key {
        call.retain_rgba(0, key, 1, output_dimensions, rgba_content, peak);
    }
    if let Some(key) = perturb_key {
        call.retain_rgba(1, key, 2, output_dimensions, match_content.unwrap(), peak);
    }
    call.retain_indexed(final_key.unwrap(), 3, output_dimensions, peak);
    let (bytes, metadata) = call.indexed_result(3);
    let result = boundary.complete(bytes, output_dimensions, metadata);
    call.finish(result)
}
