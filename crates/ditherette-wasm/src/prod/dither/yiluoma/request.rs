//! Frozen scalar request composition using existing fallible palette preparation.

use super::super::{
    ordered::BayerSize,
    placement::{placement_mask_with_converter, AdaptivePlacementRows},
};
use super::{adaptive_target, best_matched_mix, ordered_mix_index};
use crate::{
    image::{contracts::IndexedImage, ImageBuf, PaletteIndex8},
    prod::{
        color::packed::{Converter, PackedSpace},
        contract::{
            error::{DitheretteError, ErrorCode},
            request::{
                BayerSize as RequestBayerSize, DitherPolicy, DitherQuantizeRequest, Placement,
                Request,
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
    placement: Placement,
) {
    dither_yiluoma_with_progress(source, prepared, indices, size, placement, &mut [], |_| {
        Ok(())
    })
    .expect("disabled progress cannot fail");
}

/// Reports completed rows within the same ordered-mixture traversal.
/// Adaptive placement reuses converted rows when `placement_rows` holds three source-width rows;
/// empty scratch keeps the allocation-free per-pixel neighbor conversion.
pub(crate) fn dither_yiluoma_with_progress(
    source: crate::image::ImageView<'_, crate::image::Rgba8>,
    prepared: &PreparedQuantizer,
    indices: &mut [u8],
    size: BayerSize,
    placement: Placement,
    placement_rows: &mut [[f32; 3]],
    progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    let band = crate::prod::tiling::RowBand::new(0, source.dimensions().height())
        .expect("validated source height");
    dither_yiluoma_band_with_progress(
        source,
        prepared,
        indices,
        size,
        placement,
        band,
        placement_rows,
        progress,
    )
}

/// Writes one disjoint output band while retaining absolute coordinates and full-source reads.
#[allow(clippy::too_many_arguments)]
pub(super) fn dither_yiluoma_band_with_progress(
    source: crate::image::ImageView<'_, crate::image::Rgba8>,
    prepared: &PreparedQuantizer,
    indices: &mut [u8],
    size: BayerSize,
    placement: Placement,
    band: crate::prod::tiling::RowBand,
    placement_rows: &mut [[f32; 3]],
    mut progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    let dimensions = source.dimensions();
    assert!(band.y_end() <= dimensions.height());
    assert_eq!(
        indices.len(),
        band.height() as usize * dimensions.width_usize()
    );
    let space = prepared.matcher().matching.space();
    let palette = prepared.palette();
    let matcher = prepared.matcher();
    let levels = (size.width() * size.width()) as u32;
    let converter = Converter::new(PackedSpace::from_working(space));
    let mut rows = match placement {
        Placement::Adaptive { radius, .. } if !placement_rows.is_empty() => Some(
            AdaptivePlacementRows::new(source, &converter, space, radius, placement_rows),
        ),
        _ => None,
    };
    for (y, indices) in
        (band.y_start()..band.y_end()).zip(indices.chunks_exact_mut(dimensions.width_usize()))
    {
        let row = rows.as_mut().map(|rows| rows.prepare_row(y));
        for (x, index) in (0..dimensions.width()).zip(indices) {
            let pixel = source.pixel(x, y).expect("source pixel is in bounds");
            *index = match palette.prepare_pixel([pixel[0], pixel[1], pixel[2], pixel[3]]) {
                PalettePixel::Index(index) => index,
                PalettePixel::Color(rgb) => {
                    let coordinates = converter.coordinates(rgb);
                    // Everywhere has mask 1, so the target is the source and nearest is unused.
                    let target = match placement {
                        Placement::Everywhere {} => coordinates,
                        Placement::Adaptive {
                            threshold,
                            softness,
                            ..
                        } => {
                            let nearest = matcher.nearest(coordinates);
                            let mask = match &row {
                                Some(row) => row.mask_at(x, threshold, softness),
                                None => placement_mask_with_converter(
                                    source, x, y, space, placement, &converter,
                                ),
                            };
                            adaptive_target(coordinates, nearest.coordinates, mask)
                        }
                    };
                    ordered_mix_index(best_matched_mix(target, matcher, levels), x, y, size)
                }
            };
        }
        progress(y + 1)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image::{contracts::PaletteEntry, ImageDimensions, ImageView},
        prod::contract::request::{AlphaPolicy, MatchPolicy},
    };

    #[test]
    fn adaptive_rows_match_direct_neighbor_conversion() {
        let dimensions = ImageDimensions::new(9, 7).unwrap();
        let bytes = (0..9 * 7 * 4)
            .map(|i| ((i * 37 + i / 5 * 11) % 256) as u8)
            .collect::<Vec<_>>();
        let source = ImageView::packed(&bytes, dimensions).unwrap();
        let palette = [[12, 40, 200], [250, 250, 250], [0, 0, 0], [200, 60, 30]]
            .map(|rgb| PaletteEntry::Color { rgb });
        for matching in [
            MatchPolicy::SrgbEuclidean,
            MatchPolicy::OklchCircularHue,
            MatchPolicy::CielabCiede2000,
        ] {
            for alpha in [
                AlphaPolicy::Premultiplied {},
                AlphaPolicy::Preserve { threshold: 90.0 },
            ] {
                let prepared =
                    PreparedQuantizer::try_new(&palette, alpha, matching, 1 << 20).unwrap();
                for radius in [1, 2, 9] {
                    let placement = Placement::Adaptive {
                        radius,
                        threshold: 4.0,
                        softness: 3.0,
                    };
                    let run = |rows: &mut [[f32; 3]]| {
                        let mut indices = vec![0; 9 * 7];
                        dither_yiluoma_with_progress(
                            source,
                            &prepared,
                            &mut indices,
                            BayerSize::Four,
                            placement,
                            rows,
                            |_| Ok(()),
                        )
                        .unwrap();
                        indices
                    };
                    assert_eq!(
                        run(&mut [[f32::NAN; 3]; 27]),
                        run(&mut []),
                        "{matching:?} {alpha:?} radius={radius}"
                    );
                }
            }
        }
    }
}
