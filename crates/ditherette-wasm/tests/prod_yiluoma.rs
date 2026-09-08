use ditherette_wasm::{prod, spec};
use prod::{
    contract::request::MatchPolicy,
    dither::{ordered::BayerSize, yiluoma},
    quantize::matcher::{PaletteColor, PaletteMatcher},
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

#[test]
fn literal_search_matches_every_metric_and_matrix_with_original_indices_and_duplicates() {
    let colors = [
        [0; 3],
        [255; 3],
        [255, 0, 3],
        [255, 3, 0],
        [128; 3],
        [128; 3],
    ];
    for matching in POLICIES {
        let reference_matching: spec::contract::request::MatchPolicy =
            serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap();
        let reference = spec::quantize::matcher::PaletteMatcher {
            matching: reference_matching,
            colors: colors
                .iter()
                .enumerate()
                .map(|(i, &rgb)| spec::quantize::matcher::PaletteColor {
                    index: (i * 3 + 1) as u8,
                    coordinates: spec::color::rgb8_to_coordinates(rgb, reference_matching.space()),
                })
                .collect(),
        };
        let actual = PaletteMatcher {
            matching,
            colors: reference
                .colors
                .iter()
                .map(|entry| PaletteColor {
                    index: entry.index,
                    coordinates: entry.coordinates,
                })
                .collect(),
        };
        for rgb in [
            [0; 3],
            [64; 3],
            [128; 3],
            [17, 123, 241],
            [255, 1, 1],
            [255; 3],
        ] {
            let target = spec::color::rgb8_to_coordinates(rgb, reference_matching.space());
            for levels in [4, 16, 64, 256] {
                let expected = spec::dither::yiluoma::best_matched_mix(target, &reference, levels);
                let found = yiluoma::best_matched_mix(target, &actual, levels);
                assert_eq!(
                    (
                        found.low_index,
                        found.high_index,
                        found.high_ratio.to_bits()
                    ),
                    (
                        expected.low_index,
                        expected.high_index,
                        expected.high_ratio.to_bits()
                    ),
                    "{matching:?}/{levels}/{rgb:?}"
                );
            }
        }
    }
}

#[test]
fn target_endpoints_and_hue_interpolation_keep_frozen_bits() {
    let source = [0.71, 0.12, std::f32::consts::TAU - 0.1];
    let nearest = [0.4, 0.15, 0.1];
    for mask in [0.0, f32::EPSILON, 0.25, 0.5, 0.75, 1.0] {
        assert_eq!(
            yiluoma::adaptive_target(source, nearest, mask).map(f32::to_bits),
            spec::dither::yiluoma::adaptive_target(source, nearest, mask).map(f32::to_bits)
        );
    }
    assert_eq!(yiluoma::adaptive_target(source, nearest, 0.0), nearest);
    assert_eq!(yiluoma::adaptive_target(source, nearest, 1.0), source);
    assert!(yiluoma::adaptive_target(source, nearest, 0.5)[2] > 3.0);
}

#[test]
fn every_global_threshold_and_ratio_matches_frozen_strict_selection() {
    for (size, reference_size) in [
        (BayerSize::Two, spec::dither::ordered::BayerSize::Two),
        (BayerSize::Four, spec::dither::ordered::BayerSize::Four),
        (BayerSize::Eight, spec::dither::ordered::BayerSize::Eight),
        (
            BayerSize::Sixteen,
            spec::dither::ordered::BayerSize::Sixteen,
        ),
    ] {
        let levels = size.width() * size.width();
        for count in 0..=levels {
            let mix = yiluoma::PaletteMix {
                low_index: 7,
                high_index: 251,
                high_ratio: count as f32 / levels as f32,
            };
            let reference = spec::dither::yiluoma::PaletteMix {
                low_index: mix.low_index,
                high_index: mix.high_index,
                high_ratio: mix.high_ratio,
            };
            for y in (0..33).chain([u32::MAX]) {
                for x in (0..33).chain([u32::MAX]) {
                    assert_eq!(
                        yiluoma::ordered_mix_index(mix, x, y, size),
                        spec::dither::yiluoma::ordered_mix_index(reference, x, y, reference_size)
                    );
                }
            }
        }
    }
}

#[test]
fn zero_mask_retains_earlier_exact_mixture_and_first_ratio_ties() {
    let matcher = PaletteMatcher {
        matching: MatchPolicy::SrgbEuclidean,
        colors: [0.0, 128.0 / 255.0, 64.0 / 255.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| PaletteColor {
                index: index as u8,
                coordinates: [value; 3],
            })
            .collect(),
    };
    let target = yiluoma::adaptive_target([0.8; 3], matcher.colors[2].coordinates, 0.0);
    let mix = yiluoma::best_matched_mix(target, &matcher, 4);
    assert_eq!(
        mix,
        yiluoma::PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.5
        }
    );
    assert_eq!(
        [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(x, y)| yiluoma::ordered_mix_index(
            mix,
            x,
            y,
            BayerSize::Two
        )),
        [1, 0, 0, 1]
    );
    let matcher = PaletteMatcher {
        matching: MatchPolicy::SrgbEuclidean,
        colors: vec![
            PaletteColor {
                index: 0,
                coordinates: [0.0; 3],
            },
            PaletteColor {
                index: 1,
                coordinates: [1.0; 3],
            },
            PaletteColor {
                index: 2,
                coordinates: [1.0; 3],
            },
        ],
    };
    assert_eq!(
        yiluoma::best_matched_mix([0.375; 3], &matcher, 4),
        yiluoma::PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.25
        }
    );
}
