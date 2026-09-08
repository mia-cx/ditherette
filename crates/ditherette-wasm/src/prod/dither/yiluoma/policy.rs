//! Conservative recipe-class thresholds from S37's exact Chromium/Firefox complete calls.

use crate::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::request::{
            AlphaPolicy, BayerSize, DitherPolicy, MatchPolicy, Output, Placement, ResizePolicy,
            MAX_PALETTE_ENTRIES,
        },
        pipeline::{execution::RowBandPolicy, quantize::QuantizeRequest},
        tiling::WorkerBudget,
    },
};

/// Select measured configurations for supported classes; other requests remain scalar.
/// Width/height bounds are conservative thresholds, not universal crossover claims.
pub(crate) fn measured(
    output: ImageDimensions,
    request: QuantizeRequest<'_>,
    dither: DitherPolicy,
    resize: Option<Output>,
    pool: WorkerBudget,
) -> Option<RowBandPolicy> {
    let DitherPolicy::Yliluoma { size, placement } = dither else {
        return None;
    };
    if pool.pool_size() < 2 || !matches!(request.alpha, AlphaPolicy::Premultiplied {}) {
        return None;
    }
    let visible = request
        .palette
        .iter()
        .take(MAX_PALETTE_ENTRIES)
        .filter(|entry| matches!(entry, PaletteEntry::Color { .. }))
        .count();
    let (minimum_width, minimum_height, four_workers) =
        match (request.matching, size, placement, visible) {
            (MatchPolicy::SrgbEuclidean, BayerSize::Two, Placement::Everywhere {}, 2) => {
                (9, 7, false)
            }
            (MatchPolicy::SrgbEuclidean, BayerSize::Four, Placement::Everywhere {}, 4) => {
                (33, 25, false)
            }
            (MatchPolicy::SrgbEuclidean, BayerSize::Four, Placement::Adaptive { .. }, 8) => {
                (65, 49, true)
            }
            (MatchPolicy::OklchCircularHue, BayerSize::Four, Placement::Adaptive { .. }, 4)
                if matches!(
                    resize,
                    Some(Output {
                        resize: ResizePolicy::Nearest { .. },
                        ..
                    })
                ) =>
            {
                (33, 25, false)
            }
            _ => return None,
        };
    if output.width() < minimum_width || output.height() < minimum_height {
        return None;
    }
    // Pools with two or three workers use the measured two/4 fallback, never three/16.
    let (active_workers, height) = if four_workers && pool.pool_size() >= 4 {
        (4, 16)
    } else {
        (2, 4)
    };
    Some(RowBandPolicy {
        height,
        workers: WorkerBudget::new(active_workers),
        active_workers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prod::contract::request::Anchor;

    const ADAPTIVE: Placement = Placement::Adaptive {
        radius: 2,
        threshold: 5.0,
        softness: 10.0,
    };

    struct Case {
        dimensions: ImageDimensions,
        colors: usize,
        matching: MatchPolicy,
        dither: DitherPolicy,
        resize: Option<Output>,
    }

    fn cases() -> [Case; 4] {
        [
            (
                9,
                7,
                2,
                MatchPolicy::SrgbEuclidean,
                BayerSize::Two,
                Placement::Everywhere {},
                false,
            ),
            (
                33,
                25,
                4,
                MatchPolicy::SrgbEuclidean,
                BayerSize::Four,
                Placement::Everywhere {},
                false,
            ),
            (
                65,
                49,
                8,
                MatchPolicy::SrgbEuclidean,
                BayerSize::Four,
                ADAPTIVE,
                false,
            ),
            (
                33,
                25,
                4,
                MatchPolicy::OklchCircularHue,
                BayerSize::Four,
                ADAPTIVE,
                true,
            ),
        ]
        .map(
            |(width, height, colors, matching, size, placement, process)| Case {
                dimensions: ImageDimensions::new(width, height).unwrap(),
                colors,
                matching,
                dither: DitherPolicy::Yliluoma { size, placement },
                resize: process.then_some(Output {
                    width,
                    height,
                    resize: ResizePolicy::Nearest {
                        anchor: Anchor::Center,
                    },
                }),
            },
        )
    }

    fn request<'a>(case: &Case, palette: &'a [PaletteEntry]) -> QuantizeRequest<'a> {
        QuantizeRequest {
            source_width: case.dimensions.width(),
            source_height: case.dimensions.height(),
            palette,
            alpha: AlphaPolicy::Premultiplied {},
            matching: case.matching,
        }
    }

    #[test]
    fn measured_classes_keep_size_bounds_and_use_only_measured_pool_fallbacks() {
        for case in cases() {
            let palette = vec![PaletteEntry::Color { rgb: [17, 53, 211] }; case.colors];
            let request = request(&case, &palette);
            for available in [1, 2, 3, 4, 8] {
                let pool = WorkerBudget::new(available);
                let selected = measured(case.dimensions, request, case.dither, case.resize, pool);
                let expected = if available == 1 {
                    None
                } else if case.colors == 8 && available >= 4 {
                    Some((4, 16))
                } else {
                    Some((2, 4))
                };
                assert_eq!(
                    selected.map(|policy| (policy.active_workers, policy.height)),
                    expected
                );
                for smaller in [
                    ImageDimensions::new(case.dimensions.width() - 1, case.dimensions.height())
                        .unwrap(),
                    ImageDimensions::new(case.dimensions.width(), case.dimensions.height() - 1)
                        .unwrap(),
                ] {
                    assert_eq!(
                        measured(smaller, request, case.dither, case.resize, pool),
                        None
                    );
                }
            }
        }
    }

    #[test]
    fn unmeasured_alpha_cardinality_metric_and_standalone_oklch_stay_scalar() {
        for case in cases() {
            let palette = vec![PaletteEntry::Color { rgb: [31, 47, 71] }; case.colors];
            let request = request(&case, &palette);
            let pool = WorkerBudget::new(8);
            for alpha in [
                AlphaPolicy::Preserve { threshold: 128.0 },
                AlphaPolicy::Matte { rgb: [255; 3] },
            ] {
                assert_eq!(
                    measured(
                        case.dimensions,
                        QuantizeRequest { alpha, ..request },
                        case.dither,
                        case.resize,
                        pool
                    ),
                    None
                );
            }
            assert_eq!(
                measured(
                    case.dimensions,
                    QuantizeRequest {
                        matching: MatchPolicy::CielabCiede2000,
                        ..request
                    },
                    case.dither,
                    case.resize,
                    pool
                ),
                None
            );
            assert_eq!(
                measured(
                    case.dimensions,
                    QuantizeRequest {
                        palette: &palette[..palette.len() - 1],
                        ..request
                    },
                    case.dither,
                    case.resize,
                    pool
                ),
                None
            );
            assert_eq!(
                measured(
                    case.dimensions,
                    request,
                    DitherPolicy::Yliluoma {
                        size: BayerSize::Eight,
                        placement: ADAPTIVE
                    },
                    case.resize,
                    pool
                ),
                None
            );
            assert_eq!(
                measured(
                    case.dimensions,
                    request,
                    DitherPolicy::None {},
                    case.resize,
                    pool
                ),
                None
            );
            if case.resize.is_some() {
                assert_eq!(
                    measured(case.dimensions, request, case.dither, None, pool),
                    None
                );
            }
        }
    }

    #[test]
    fn palette_bytes_do_not_select_policy_and_only_retained_visible_entries_count() {
        let case = &cases()[1];
        let mut palette = vec![PaletteEntry::Transparent {}; MAX_PALETTE_ENTRIES + 1];
        palette[..4].fill(PaletteEntry::Color { rgb: [255; 3] });
        palette[MAX_PALETTE_ENTRIES] = PaletteEntry::Color { rgb: [0; 3] };
        let pool = WorkerBudget::new(4);
        let selected = measured(
            case.dimensions,
            request(case, &palette),
            case.dither,
            case.resize,
            pool,
        );
        assert_eq!(
            selected.map(|policy| (policy.active_workers, policy.height)),
            Some((2, 4))
        );
        palette[..4].fill(PaletteEntry::Color { rgb: [0; 3] });
        assert_eq!(
            measured(
                case.dimensions,
                request(case, &palette),
                case.dither,
                case.resize,
                pool
            ),
            selected
        );
    }
}
