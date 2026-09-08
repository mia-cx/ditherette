//! Frozen scalar request composition using existing fallible palette preparation.

use super::super::{ordered::BayerSize, placement::placement_mask_at};
use super::{adaptive_target, best_matched_mix, ordered_mix_index};
use crate::{
    image::{contracts::IndexedImage, ImageBuf, PaletteIndex8},
    prod::{
        color::packed::{rgb8_to_coordinates, Converter},
        contract::{
            error::{DitheretteError, ErrorCode},
            request::{
                BayerSize as RequestBayerSize, DitherPolicy, DitherQuantizeRequest, Request,
            },
        },
        palette::{allocation::Budget, PalettePixel, PreparationError},
        quantize::{PreparedQuantizer, QuantizeError},
    },
};
use std::mem::size_of;

/// Validates and executes the frozen scalar Yliluoma recipe with bounded owned allocations.
/// Source bytes are borrowed. Preparation, metadata, indices, and one temporary converter count toward the limit.
pub fn dither_yiluoma(
    request: DitherQuantizeRequest<'_>,
    memory_limit: u64,
) -> Result<IndexedImage, QuantizeError> {
    let layout = Request::DitherAndQuantize(request)
        .validate()
        .map_err(QuantizeError::Request)?;
    let DitherPolicy::Yliluoma { size, placement } = request.dither else {
        return Err(QuantizeError::Request(DitheretteError::new(
            ErrorCode::UnsupportedOperation,
            "dither.family",
            "This implementation requires the Yliluoma family.",
        )));
    };
    let size = match size {
        RequestBayerSize::Two => BayerSize::Two,
        RequestBayerSize::Four => BayerSize::Four,
        RequestBayerSize::Eight => BayerSize::Eight,
        RequestBayerSize::Sixteen => BayerSize::Sixteen,
    };
    let quantize = request.quantize;
    let count = layout.output.pixel_count().expect("validated dimensions");
    let fixed = (size_of::<ImageBuf<PaletteIndex8>>() + size_of::<Converter>()) as u64;
    let preparation_limit = memory_limit
        .checked_sub(fixed + count as u64)
        .ok_or(QuantizeError::Preparation(PreparationError::memory()))?;
    let prepared = PreparedQuantizer::try_new(
        quantize.palette,
        quantize.alpha,
        quantize.matching,
        preparation_limit,
    )
    .map_err(QuantizeError::Preparation)?;
    let mut budget = Budget::new(memory_limit, prepared.capacity_bytes() + fixed)
        .map_err(QuantizeError::Preparation)?;
    let mut indices = Vec::new();
    budget
        .reserve(&mut indices, count)
        .map_err(QuantizeError::Preparation)?;
    indices.resize(count, 0);
    dither_yiluoma_into(layout.source, &prepared, &mut indices, size, placement);
    let indices = ImageBuf::<PaletteIndex8>::from_vec_packed(indices, layout.output)
        .expect("validated dimensions and reserved index length");
    Ok(prepared.into_indexed(indices))
}

/// Writes the literal scalar recipe into validated caller-owned index storage without allocation.
/// Alpha preparation and adaptive placement retain their distinct source-byte inputs.
pub fn dither_yiluoma_into(
    source: crate::image::ImageView<'_, crate::image::Rgba8>,
    prepared: &PreparedQuantizer,
    indices: &mut [u8],
    size: BayerSize,
    placement: crate::prod::contract::request::Placement,
) {
    let dimensions = source.dimensions();
    assert_eq!(
        indices.len(),
        dimensions.pixel_count().expect("valid dimensions")
    );
    let matching = prepared.matcher().matching;
    let palette = prepared.palette();
    let matcher = prepared.matcher();
    for y in 0..dimensions.height() {
        for x in 0..dimensions.width() {
            let pixel = source.pixel(x, y).expect("source pixel is in bounds");
            let rgba = [pixel[0], pixel[1], pixel[2], pixel[3]];
            let index = match palette.prepare_pixel(rgba) {
                PalettePixel::Index(index) => index,
                PalettePixel::Color(rgb) => {
                    let coordinates = rgb8_to_coordinates(rgb, matching.space());
                    let nearest = matcher.nearest(coordinates);
                    let mask = placement_mask_at(source, x, y, matching.space(), placement);
                    let target = adaptive_target(coordinates, nearest.coordinates, mask);
                    let mix =
                        best_matched_mix(target, matcher, (size.width() * size.width()) as u32);
                    ordered_mix_index(mix, x, y, size)
                }
            };
            indices[y as usize * dimensions.width_usize() + x as usize] = index;
        }
    }
}
