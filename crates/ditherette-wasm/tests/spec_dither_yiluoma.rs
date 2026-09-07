use ditherette_wasm::spec::{
    contract::request::MatchPolicy,
    dither::yiluoma::{best_matched_mix, best_ordered_mix, PaletteMix},
    quantize::matcher::{PaletteColor, PaletteMatcher},
};

use ditherette_wasm::{
    image::contracts::{PaletteEntry, WarningCode},
    spec::{
        contract::{
            error::ErrorCode,
            request::{
                AlphaPolicy, BayerSize, DitherPolicy, DitherQuantizeRequest, Placement,
                QuantizeRequest, Source,
            },
        },
        dither::yiluoma::{adaptive_target, dither_yiluoma},
    },
};

const POLICIES: [MatchPolicy; 15] = [
    MatchPolicy::SrgbEuclidean,
    MatchPolicy::SrgbCompuphase,
    MatchPolicy::SrgbRec601,
    MatchPolicy::SrgbRec709,
    MatchPolicy::LinearRgbEuclidean,
    MatchPolicy::OklabEuclidean,
    MatchPolicy::OklchEuclidean,
    MatchPolicy::OklchCircularHue,
    MatchPolicy::OklchHueArc,
    MatchPolicy::CielabEuclidean,
    MatchPolicy::CielabCiede2000,
    MatchPolicy::CielchEuclidean,
    MatchPolicy::CielchCircularHue,
    MatchPolicy::CielchHueArc,
    MatchPolicy::YcbcrEuclidean,
];

fn request<'a>(
    data: &'a [u8],
    width: u32,
    height: u32,
    palette: &'a [PaletteEntry],
) -> DitherQuantizeRequest<'a> {
    DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source: Source {
                width,
                height,
                data,
            },
            palette,
            alpha: AlphaPolicy::Preserve { threshold: 0.0 },
            matching: MatchPolicy::SrgbEuclidean,
        },
        dither: DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Everywhere,
        },
    }
}

fn matcher(colors: &[(u8, [f32; 3])], matching: MatchPolicy) -> PaletteMatcher {
    PaletteMatcher {
        colors: colors
            .iter()
            .map(|&(index, coordinates)| PaletteColor { index, coordinates })
            .collect(),
        matching,
    }
}

#[test]
fn pair_and_ratio_ties_keep_the_first_enumerated_candidate() {
    let palette = [[0.0; 3], [1.0; 3], [1.0; 3]];
    assert_eq!(
        best_ordered_mix([0.5; 3], &palette, 4),
        PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.5
        }
    );
    // 3/8 is equally far from ratios 1/4 and 1/2. Earlier ratio 1/4 wins.
    assert_eq!(
        best_ordered_mix([0.375; 3], &palette, 4),
        PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.25
        }
    );
    // The initialized first entry wins a tie with the first nonzero mixture.
    assert_eq!(
        best_ordered_mix([0.125; 3], &palette, 4),
        PaletteMix {
            low_index: 0,
            high_index: 0,
            high_ratio: 0.0
        }
    );
}

#[test]
fn matched_search_returns_original_indices_and_preserves_an_earlier_exact_pair() {
    let matcher = matcher(
        &[(1, [0.0; 3]), (3, [1.0; 3]), (5, [0.5; 3])],
        MatchPolicy::SrgbEuclidean,
    );
    assert_eq!(matcher.nearest([0.5; 3]).index, 5);
    assert_eq!(
        best_matched_mix([0.5; 3], &matcher, 4),
        PaletteMix {
            low_index: 1,
            high_index: 3,
            high_ratio: 0.5
        }
    );
}

#[test]
fn hue_interpolation_takes_the_componentwise_midpoint_not_the_shortest_arc() {
    for matching in [
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
    ] {
        let matcher = matcher(
            &[
                (2, [0.5, 0.2, 0.1]),
                (7, [0.5, 0.2, std::f32::consts::TAU - 0.1]),
            ],
            matching,
        );
        // The inherited midpoint hue is PI, opposite the shortest-arc midpoint near zero.
        assert_eq!(
            best_matched_mix([0.5, 0.2, std::f32::consts::PI], &matcher, 4),
            PaletteMix {
                low_index: 2,
                high_index: 7,
                high_ratio: 0.5
            },
            "{matching:?}"
        );
    }
}

#[test]
fn neutral_hue_is_ignored_only_when_the_selected_metric_ignores_it() {
    for matching in [
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
    ] {
        let matcher = matcher(
            &[
                (2, [0.5, 0.0, 0.1]),
                (7, [0.5, 0.0, std::f32::consts::TAU - 0.1]),
            ],
            matching,
        );
        assert_eq!(
            best_matched_mix([0.5, 0.0, std::f32::consts::PI], &matcher, 4),
            PaletteMix {
                low_index: 2,
                high_index: 2,
                high_ratio: 0.0
            }
        );
    }
    let matcher = matcher(
        &[
            (2, [0.5, 0.0, 0.1]),
            (7, [0.5, 0.0, std::f32::consts::TAU - 0.1]),
        ],
        MatchPolicy::OklchEuclidean,
    );
    assert_eq!(
        best_matched_mix([0.5, 0.0, std::f32::consts::PI], &matcher, 4).high_ratio,
        0.5
    );
}

#[test]
fn adaptive_target_keeps_exact_endpoints_and_interpolates_hue_componentwise() {
    let source = [0.25, 0.1, 0.1];
    let nearest = [0.75, 0.3, std::f32::consts::TAU - 0.1];
    assert_eq!(adaptive_target(source, nearest, 0.0), nearest);
    assert_eq!(adaptive_target(source, nearest, 1.0), source);
    let midpoint = adaptive_target(source, nearest, 0.5);
    assert_eq!(midpoint[0], 0.5);
    assert!((midpoint[1] - 0.2).abs() < 1e-7);
    assert!((midpoint[2] - std::f32::consts::PI).abs() < 1e-6);
}

#[test]
fn zero_full_and_half_masks_produce_the_calculated_bayer_patterns() {
    let data = [128, 128, 128, 255].repeat(4);
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let base = request(&data, 2, 2, &palette);
    assert_eq!(dither_yiluoma(base).unwrap().indices.data(), &[1, 0, 0, 1]);
    for (threshold, softness, expected) in [
        (1.0, 0.0, [1, 1, 1, 1]),
        (0.0, 0.0, [1, 0, 0, 1]),
        (0.0, 1.0, [1, 1, 0, 1]),
    ] {
        let adapted = DitherQuantizeRequest {
            dither: DitherPolicy::Yliluoma {
                size: BayerSize::Two,
                placement: Placement::Adaptive {
                    radius: 1,
                    threshold,
                    softness,
                },
            },
            ..base
        };
        assert_eq!(dither_yiluoma(adapted).unwrap().indices.data(), &expected);
    }
}

#[test]
fn zero_mask_preserves_the_earlier_exact_pair_instead_of_forcing_the_nearest_entry() {
    let data = [64, 64, 64, 255].repeat(4);
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [128; 3] },
        PaletteEntry::Color { rgb: [64; 3] },
    ];
    let mut input = request(&data, 2, 2, &palette);
    input.dither = DitherPolicy::Yliluoma {
        size: BayerSize::Two,
        placement: Placement::Adaptive {
            radius: 1,
            threshold: 1.0,
            softness: 0.0,
        },
    };
    assert_eq!(dither_yiluoma(input).unwrap().indices.data(), &[1, 0, 0, 1]);
}

#[test]
fn all_matching_policies_and_bayer_sizes_keep_stable_visible_and_transparent_indices() {
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let data = [
        255, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 255,
    ];
    for matching in POLICIES {
        for size in [
            BayerSize::Two,
            BayerSize::Four,
            BayerSize::Eight,
            BayerSize::Sixteen,
        ] {
            let mut input = request(&data, 2, 2, &palette);
            input.quantize.matching = matching;
            input.dither = DitherPolicy::Yliluoma {
                size,
                placement: Placement::Everywhere,
            };
            let result = dither_yiluoma(input).unwrap();
            assert_eq!(
                result.indices.data(),
                &[2, 4, 1, 0],
                "{matching:?} {size:?}"
            );
            assert_eq!(result.palette.transparent_index, Some(1));
            assert_eq!(result.palette.rgba.len(), 20);
            assert!(result.warnings.is_empty());
        }
    }
}

#[test]
fn alpha_preparation_precedes_matching_and_transparent_only_bypasses_search() {
    let palette = [
        PaletteEntry::Color {
            rgb: [200, 100, 50],
        },
        PaletteEntry::Color { rgb: [100, 50, 25] },
        PaletteEntry::Color {
            rgb: [100, 50, 152],
        },
        PaletteEntry::Transparent {},
    ];
    let data = [200, 100, 50, 128];
    for matching in POLICIES {
        for (alpha, expected) in [
            (
                AlphaPolicy::Preserve {
                    threshold: 127.9999999,
                },
                0,
            ),
            (AlphaPolicy::Preserve { threshold: 128.0 }, 3),
            (AlphaPolicy::Premultiplied, 1),
            (AlphaPolicy::Matte { rgb: [0, 0, 255] }, 2),
        ] {
            let mut input = request(&data, 1, 1, &palette);
            input.quantize.matching = matching;
            input.quantize.alpha = alpha;
            assert_eq!(
                dither_yiluoma(input).unwrap().indices.data(),
                &[expected],
                "{matching:?} {alpha:?}"
            );
        }
    }
    let transparent = [PaletteEntry::Transparent {}, PaletteEntry::Transparent {}];
    for alpha in [
        AlphaPolicy::Preserve { threshold: 0.0 },
        AlphaPolicy::Premultiplied,
        AlphaPolicy::Matte { rgb: [255; 3] },
    ] {
        let mut input = request(&data, 1, 1, &transparent);
        input.quantize.alpha = alpha;
        let result = dither_yiluoma(input).unwrap();
        assert_eq!(result.indices.data(), &[0]);
        assert_eq!(result.warnings[0].code, WarningCode::TransparentOnly);
    }
}

#[test]
fn truncation_and_transparent_fallback_preserve_warning_order_and_index_255() {
    let mut palette = vec![PaletteEntry::Color { rgb: [0; 3] }; 255];
    palette.push(PaletteEntry::Color { rgb: [255, 0, 0] });
    palette.push(PaletteEntry::Transparent {});
    let data = [255, 0, 0, 255, 255, 0, 0, 0];
    let result = dither_yiluoma(request(&data, 2, 1, &palette)).unwrap();
    assert_eq!(result.indices.data(), &[255, 0]);
    assert_eq!(result.palette.rgba.len(), 1024);
    assert_eq!(
        result
            .warnings
            .iter()
            .map(|warning| warning.code)
            .collect::<Vec<_>>(),
        vec![
            WarningCode::PaletteTruncated,
            WarningCode::TransparentFallback
        ]
    );
}

#[test]
fn typed_composition_rejects_invalid_or_wrong_family_requests_without_mutation() {
    let data = [10, 20, 30, 255];
    let palette = [PaletteEntry::Color { rgb: [0; 3] }];
    let base = request(&data, 1, 1, &palette);
    let wrong_family = DitherQuantizeRequest {
        dither: DitherPolicy::None,
        ..base
    };
    let error = dither_yiluoma(wrong_family).unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::UnsupportedOperation, "dither.family")
    );
    let mut invalid = base;
    invalid.quantize.palette = &[];
    let error = dither_yiluoma(invalid).unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidPalette, "palette")
    );
    invalid = base;
    invalid.dither = DitherPolicy::Yliluoma {
        size: BayerSize::Two,
        placement: Placement::Adaptive {
            radius: 0,
            threshold: 0.0,
            softness: 0.0,
        },
    };
    assert_eq!(
        dither_yiluoma(invalid).unwrap_err().path,
        "dither.placement.radius"
    );
    assert_eq!(data, [10, 20, 30, 255]);
}

#[test]
fn every_bayer_size_realizes_an_exact_half_mix_with_global_matrix_orientation() {
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [128; 3] },
    ];
    for (size, width) in [
        (BayerSize::Two, 2),
        (BayerSize::Four, 4),
        (BayerSize::Eight, 8),
        (BayerSize::Sixteen, 16),
    ] {
        let data = [64, 64, 64, 255].repeat((width * width) as usize);
        let mut input = request(&data, width, width, &palette);
        input.dither = DitherPolicy::Yliluoma {
            size,
            placement: Placement::Everywhere,
        };
        let result = dither_yiluoma(input).unwrap();
        assert_eq!(
            result
                .indices
                .data()
                .iter()
                .filter(|&&index| index == 1)
                .count(),
            (width * width / 2) as usize
        );
        for y in 0..width {
            for x in 0..width {
                // The first Bayer recursion bit selects the half-tile checkerboard for every width.
                assert_eq!(
                    result.indices.data()[(y * width + x) as usize],
                    u8::from(x % 2 == y % 2)
                );
            }
        }
    }
}
