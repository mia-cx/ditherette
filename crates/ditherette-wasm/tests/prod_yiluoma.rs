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
fn public_fixture_requests_match_native_literal() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../packages/ditherette/tests/fixtures/yiluoma.json"
    ))
    .unwrap();
    for (index, case) in fixtures["cases"].as_array().unwrap().iter().enumerate() {
        let raw = &case["request"];
        let data: Vec<u8> = serde_json::from_value(raw["source"]["data"].clone()).unwrap();
        let palette =
            serde_json::from_value::<Vec<ditherette_wasm::image::contracts::PaletteEntry>>(
                raw["palette"].clone(),
            )
            .unwrap();
        let mut request = request(
            &data,
            raw["source"]["width"].as_u64().unwrap() as u32,
            raw["source"]["height"].as_u64().unwrap() as u32,
            &palette,
        );
        request.quantize.alpha = serde_json::from_value(raw["alpha"].clone()).unwrap();
        request.quantize.matching = serde_json::from_value(raw["matching"].clone()).unwrap();
        request.dither = serde_json::from_value(raw["dither"].clone()).unwrap();
        let actual = yiluoma::dither_yiluoma(request, 1 << 24).unwrap();
        let expected: Vec<u8> = serde_json::from_value(case["output"]["indices"].clone()).unwrap();
        assert_eq!(actual.indices.data(), expected, "fixture {index}: {raw}");
    }
}

fn request<'a>(
    data: &'a [u8],
    width: u32,
    height: u32,
    palette: &'a [ditherette_wasm::image::contracts::PaletteEntry],
) -> prod::contract::request::DitherQuantizeRequest<'a> {
    use prod::contract::request::*;
    DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source: Source {
                width,
                height,
                data,
            },
            palette,
            alpha: AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            matching: MatchPolicy::SrgbEuclidean,
        },
        dither: DitherPolicy::Yliluoma {
            size: prod::contract::request::BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    }
}

fn oracle(
    request: prod::contract::request::DitherQuantizeRequest<'_>,
) -> ditherette_wasm::image::contracts::IndexedImage {
    use spec::contract::request::*;
    spec::dither::yiluoma::dither_yiluoma(DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: request.quantize.version,
            source: Source {
                width: request.quantize.source.width,
                height: request.quantize.source.height,
                data: request.quantize.source.data,
            },
            palette: request.quantize.palette,
            alpha: serde_json::from_value(serde_json::to_value(request.quantize.alpha).unwrap())
                .unwrap(),
            matching: serde_json::from_value(
                serde_json::to_value(request.quantize.matching).unwrap(),
            )
            .unwrap(),
        },
        dither: serde_json::from_value(serde_json::to_value(request.dither).unwrap()).unwrap(),
    })
    .unwrap()
}

#[test]
fn scalar_request_matches_frozen_alpha_placement_and_complete_metadata_for_every_metric() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    use prod::contract::request::{AlphaPolicy, BayerSize, DitherPolicy, Placement};
    let data = [
        255, 0, 0, 0, 17, 33, 71, 127, 128, 128, 128, 128, 0, 0, 255, 255, 0, 255, 0, 1, 128, 17,
        255, 254,
    ];
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255, 3, 0] },
        PaletteEntry::Color { rgb: [255, 0, 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    for matching in POLICIES {
        for size in [
            BayerSize::Two,
            BayerSize::Four,
            BayerSize::Eight,
            BayerSize::Sixteen,
        ] {
            for alpha in [
                AlphaPolicy::Preserve {
                    threshold: 127.9999999,
                },
                AlphaPolicy::Premultiplied {},
                AlphaPolicy::Matte { rgb: [29, 71, 211] },
            ] {
                for placement in [
                    Placement::Everywhere {},
                    Placement::Adaptive {
                        radius: 1,
                        threshold: 5.0,
                        softness: 10.0,
                    },
                    Placement::Adaptive {
                        radius: 32768,
                        threshold: 100.0,
                        softness: 0.0,
                    },
                ] {
                    let mut input = request(&data, 3, 2, &palette);
                    input.quantize.matching = matching;
                    input.quantize.alpha = alpha;
                    input.dither = DitherPolicy::Yliluoma { size, placement };
                    assert_eq!(
                        yiluoma::dither_yiluoma(input, u64::MAX).unwrap(),
                        oracle(input),
                        "{matching:?}/{size:?}/{alpha:?}/{placement:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn request_alpha_bypass_truncation_and_exact_budget_reuse_bounded_palette_storage() {
    use ditherette_wasm::image::{contracts::PaletteEntry, ImageBuf, PaletteIndex8};
    use prod::{
        color::packed::Converter,
        contract::error::ErrorCode,
        quantize::{PreparedQuantizer, QuantizeError},
    };
    let source = [64, 64, 64, 255];
    for palette in [
        vec![PaletteEntry::Transparent {}],
        vec![PaletteEntry::Color { rgb: [0; 3] }],
        (0..257)
            .map(|i| PaletteEntry::Color { rgb: [i as u8; 3] })
            .collect(),
    ] {
        let input = request(&source, 1, 1, &palette);
        let prepared = PreparedQuantizer::required_capacity_bytes(
            &palette,
            input.quantize.alpha,
            input.quantize.matching,
        )
        .unwrap();
        let limit = prepared
            + std::mem::size_of::<Converter>() as u64
            + std::mem::size_of::<ImageBuf<PaletteIndex8>>() as u64
            + 1;
        assert_eq!(
            yiluoma::dither_yiluoma(input, limit).unwrap(),
            oracle(input)
        );
        let error = yiluoma::dither_yiluoma(input, limit - 1).unwrap_err();
        assert!(
            matches!(error, QuantizeError::Preparation(error) if error.code == ErrorCode::MemoryLimit)
        );
        assert!(yiluoma::dither_yiluoma(input, limit).is_ok());
    }
}

#[test]
fn request_validation_precedes_preparation_and_unsupported_families() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    use prod::{
        contract::{error::ErrorCode, request::DitherPolicy},
        quantize::QuantizeError,
    };
    let palette = [PaletteEntry::Color { rgb: [0; 3] }];
    let mut input = request(&[0; 4], 1, 1, &palette);
    input.dither = DitherPolicy::None {};
    assert!(
        matches!(yiluoma::dither_yiluoma(input, 0), Err(QuantizeError::Request(error)) if error.code == ErrorCode::UnsupportedOperation && error.path == "dither.family")
    );
    input.quantize.source.width = 2;
    assert!(
        matches!(yiluoma::dither_yiluoma(input, 0), Err(QuantizeError::Request(error)) if error.code == ErrorCode::InvalidImage)
    );
}

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
