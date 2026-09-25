//! Frozen scalar request composition using existing fallible palette preparation.

use super::super::{
    ordered::BayerSize,
    placement::{placement_mask_with_converter, AdaptivePlacementRows},
};
use super::index::MixIndex;
use super::{adaptive_target, best_matched_mix, ordered_mix_index, MixCache};
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
    let index = (count >= super::index::MIN_INDEX_PIXELS && prepared.can_match_rgb())
        .then(|| {
            MixIndex::try_new(
                prepared.matcher(),
                (size.width() * size.width()) as u32,
                memory_limit - budget.used,
            )
        })
        .flatten();
    dither_yiluoma_with_progress_indexed(
        layout.source,
        &prepared,
        &mut indices,
        size,
        placement,
        &mut [],
        &mut [],
        index.as_ref(),
        |_| Ok(()),
    )
    .expect("disabled progress cannot fail");
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
    dither_yiluoma_with_progress(
        source,
        prepared,
        indices,
        size,
        placement,
        &mut [],
        &mut [],
        |_| Ok(()),
    )
    .expect("disabled progress cannot fail");
}

/// Reports completed rows within the same ordered-mixture traversal.
/// Optional scratch never changes output. Adaptive placement reuses converted rows when
/// `placement_rows` holds three source-width rows; Everywhere memoizes mixtures when `mixes`
/// holds a power-of-two table. Empty scratch keeps the allocation-free direct path.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dither_yiluoma_with_progress(
    source: crate::image::ImageView<'_, crate::image::Rgba8>,
    prepared: &PreparedQuantizer,
    indices: &mut [u8],
    size: BayerSize,
    placement: Placement,
    placement_rows: &mut [[f32; 3]],
    mixes: &mut [u64],
    progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    dither_yiluoma_with_progress_indexed(
        source,
        prepared,
        indices,
        size,
        placement,
        placement_rows,
        mixes,
        None,
        progress,
    )
}

/// Uses optional caller-accounted index storage; an absent index keeps the literal scan.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dither_yiluoma_with_progress_indexed(
    source: crate::image::ImageView<'_, crate::image::Rgba8>,
    prepared: &PreparedQuantizer,
    indices: &mut [u8],
    size: BayerSize,
    placement: Placement,
    placement_rows: &mut [[f32; 3]],
    mixes: &mut [u64],
    mix_index: Option<&MixIndex>,
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
        mixes,
        mix_index,
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
    mixes: &mut [u64],
    mix_index: Option<&MixIndex>,
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
    let mut mixes = match placement {
        Placement::Everywhere {} if !mixes.is_empty() => Some(MixCache::new(mixes, levels)),
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
                    let search = || {
                        let coordinates = converter.coordinates(rgb);
                        mix_index.map_or_else(
                            || best_matched_mix(coordinates, matcher, levels),
                            |index| index.best(coordinates, matcher),
                        )
                    };
                    // Everywhere has mask 1, so the target is the source and nearest is unused.
                    let mix = match placement {
                        Placement::Everywhere {} => match &mut mixes {
                            Some(mixes) => mixes.mix(rgb, search),
                            None => search(),
                        },
                        Placement::Adaptive {
                            threshold,
                            softness,
                            ..
                        } => {
                            let coordinates = converter.coordinates(rgb);
                            let nearest = matcher.nearest(coordinates);
                            let mask = match &row {
                                Some(row) => row.mask_at(x, threshold, softness),
                                None => placement_mask_with_converter(
                                    source, x, y, space, placement, &converter,
                                ),
                            };
                            let target = adaptive_target(coordinates, nearest.coordinates, mask);
                            mix_index.map_or_else(
                                || best_matched_mix(target, matcher, levels),
                                |index| index.best(target, matcher),
                            )
                        }
                    };
                    ordered_mix_index(mix, x, y, size)
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
                            &mut [],
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

    #[test]
    fn memoized_everywhere_mixtures_match_direct_search() {
        let dimensions = ImageDimensions::new(40, 31).unwrap();
        // Few distinct colors repeat across the image; alpha varies so preparation matters.
        let bytes = (0..40 * 31)
            .flat_map(|i| {
                let c = (i % 23) as u8;
                [
                    c * 11,
                    255 - c * 7,
                    c * 5 + 40,
                    if i % 7 == 0 { 90 } else { 255 },
                ]
            })
            .collect::<Vec<_>>();
        let source = ImageView::packed(&bytes, dimensions).unwrap();
        let palette = [
            [12, 40, 200],
            [250, 250, 250],
            [0, 0, 0],
            [200, 60, 30],
            [90, 200, 90],
        ]
        .map(|rgb| PaletteEntry::Color { rgb });
        for matching in [
            MatchPolicy::SrgbEuclidean,
            MatchPolicy::SrgbRec709,
            MatchPolicy::OklchHueArc,
            MatchPolicy::CielabCiede2000,
        ] {
            for alpha in [
                AlphaPolicy::Premultiplied {},
                AlphaPolicy::Preserve { threshold: 100.0 },
            ] {
                let prepared =
                    PreparedQuantizer::try_new(&palette, alpha, matching, 1 << 20).unwrap();
                for size in [BayerSize::Two, BayerSize::Eight, BayerSize::Sixteen] {
                    let run = |mixes: &mut [u64]| {
                        let mut indices = vec![0; 40 * 31];
                        dither_yiluoma_with_progress(
                            source,
                            &prepared,
                            &mut indices,
                            size,
                            Placement::Everywhere {},
                            &mut [],
                            mixes,
                            |_| Ok(()),
                        )
                        .unwrap();
                        indices
                    };
                    let direct = run(&mut []);
                    // A tiny table forces collisions; stale scratch must not leak into results.
                    assert_eq!(run(&mut [u64::MAX; 4]), direct, "{matching:?} {alpha:?}");
                    assert_eq!(run(&mut vec![0; 1024]), direct, "{matching:?} {alpha:?}");
                }
            }
        }
    }

    #[test]
    fn indexed_pipeline_matches_literal_with_alpha_and_adaptive_placement() {
        let dimensions = ImageDimensions::new(13, 9).unwrap();
        let bytes = (0..13 * 9)
            .flat_map(|i| {
                [
                    (i * 71) as u8,
                    (i * 37 + 8) as u8,
                    (i * 113 + 12) as u8,
                    [0, 127, 255][i % 3],
                ]
            })
            .collect::<Vec<_>>();
        let source = ImageView::packed(&bytes, dimensions).unwrap();
        let palette = [
            PaletteEntry::Color { rgb: [0, 0, 0] },
            PaletteEntry::Color {
                rgb: [255, 255, 255],
            },
            PaletteEntry::Transparent {},
            PaletteEntry::Color { rgb: [220, 40, 80] },
            PaletteEntry::Color {
                rgb: [20, 190, 230],
            },
        ];
        for alpha in [
            AlphaPolicy::Preserve { threshold: 127.0 },
            AlphaPolicy::Premultiplied {},
        ] {
            let prepared =
                PreparedQuantizer::try_new(&palette, alpha, MatchPolicy::OklabEuclidean, 1 << 20)
                    .unwrap();
            for size in [
                BayerSize::Two,
                BayerSize::Four,
                BayerSize::Eight,
                BayerSize::Sixteen,
            ] {
                let levels = (size.width() * size.width()) as u32;
                let index = MixIndex::try_new(prepared.matcher(), levels, 1 << 20).unwrap();
                for placement in [
                    Placement::Everywhere {},
                    Placement::Adaptive {
                        radius: 2,
                        threshold: 4.0,
                        softness: 3.0,
                    },
                ] {
                    let mut expected = vec![0; 13 * 9];
                    let mut actual = expected.clone();
                    dither_yiluoma_with_progress(
                        source,
                        &prepared,
                        &mut expected,
                        size,
                        placement,
                        &mut [],
                        &mut [],
                        |_| Ok(()),
                    )
                    .unwrap();
                    dither_yiluoma_with_progress_indexed(
                        source,
                        &prepared,
                        &mut actual,
                        size,
                        placement,
                        &mut [],
                        &mut [],
                        Some(&index),
                        |_| Ok(()),
                    )
                    .unwrap();
                    assert_eq!(actual, expected, "{alpha:?} {size:?} {placement:?}");
                }
            }
        }
    }
}
