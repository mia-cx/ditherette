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

#[test]
fn oklab_index_uses_spare_budget_and_low_budget_keeps_literal_output() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    use prod::contract::request::{AlphaPolicy, BayerSize, DitherPolicy, Placement};
    let width = 40;
    let height = 32;
    let data = (0..width * height)
        .flat_map(|pixel| {
            [
                (pixel * 17) as u8,
                (pixel * 39 + 1) as u8,
                (pixel * 73 + 2) as u8,
                255,
            ]
        })
        .collect::<Vec<_>>();
    let palette = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
        PaletteEntry::Color { rgb: [220, 50, 30] },
        PaletteEntry::Color { rgb: [20, 200, 90] },
        PaletteEntry::Color { rgb: [40, 80, 230] },
    ];
    let mut request = request(&data, width, height, &palette);
    request.quantize.alpha = AlphaPolicy::Premultiplied {};
    request.quantize.matching = MatchPolicy::OklabEuclidean;
    request.dither = DitherPolicy::Yliluoma {
        size: BayerSize::Four,
        placement: Placement::Everywhere {},
    };
    let low = yiluoma::dither_yiluoma(request, 5_000).unwrap();
    let high = yiluoma::dither_yiluoma(request, 1 << 20).unwrap();
    assert_eq!(low.indices.data(), high.indices.data());
    assert_eq!(low.palette, high.palette);
    assert_eq!(low.warnings, high.warnings);
}

#[test]
fn indexed_requests_match_frozen_output_for_every_policy_size_and_edge_palette() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    use prod::contract::request::{AlphaPolicy, BayerSize, DitherPolicy, Placement};

    let palette = (0..31)
        .map(|index| {
            let rgb = if index >= 27 {
                let grey = 118 + index as u8 - 27;
                [grey, grey, grey.wrapping_add((index & 1) as u8)]
            } else {
                [
                    (index * 8) as u8,
                    ((index * 57) % 256) as u8,
                    ((index * 91) % 256) as u8,
                ]
            };
            PaletteEntry::Color { rgb }
        })
        .chain([
            PaletteEntry::Color { rgb: [20, 30, 40] },
            PaletteEntry::Color { rgb: [20, 30, 40] },
            PaletteEntry::Transparent {},
        ])
        .collect::<Vec<_>>();
    let mut state = 0x9183_26a5u32;
    for matching in POLICIES {
        for size in [
            BayerSize::Two,
            BayerSize::Four,
            BayerSize::Eight,
            BayerSize::Sixteen,
        ] {
            let samples = if matches!(size, BayerSize::Two | BayerSize::Four) {
                300
            } else {
                20
            };
            let mut data = vec![0; 32 * 32 * 4];
            for (sample, pixel) in data.chunks_exact_mut(4).take(samples).enumerate() {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let rgb = if sample % 11 == 0 {
                    let grey = state as u8;
                    [grey, grey, grey.wrapping_add((sample & 1) as u8)]
                } else {
                    [state as u8, (state >> 8) as u8, (state >> 16) as u8]
                };
                pixel.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
            let mut input = request(&data, 32, 32, &palette);
            input.quantize.alpha = AlphaPolicy::Preserve { threshold: 127.0 };
            input.quantize.matching = matching;
            input.dither = DitherPolicy::Yliluoma {
                size,
                placement: Placement::Everywhere {},
            };
            assert_eq!(
                yiluoma::dither_yiluoma(input, 1 << 29).unwrap(),
                oracle(input),
                "{matching:?}/{size:?}"
            );
        }
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

struct PipelineBoundary<'a>(&'a [u8]);

impl prod::pipeline::quantize::InputBoundary for PipelineBoundary<'_> {
    fn input_len(&mut self) -> Result<usize, prod::contract::failure::Failure> {
        Ok(self.0.len())
    }

    fn copy_input(
        &mut self,
        destination: &mut [u8],
    ) -> Result<(), prod::contract::failure::Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
}

impl prod::pipeline::quantize::QuantizeBoundary for PipelineBoundary<'_> {
    type Output = Vec<u8>;

    fn complete(
        &mut self,
        indices: &[u8],
        _dimensions: ditherette_wasm::image::ImageDimensions,
        _palette: prod::pipeline::quantize::IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, prod::contract::failure::Failure> {
        Ok(indices.to_vec())
    }
}

#[test]
fn processor_tight_budget_keeps_index_and_skips_rgb_cache() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    use prod::{
        contract::request::{AlphaPolicy, BayerSize, DitherPolicy, Placement},
        pipeline::{processor::Processor, quantize::QuantizeRequest},
    };

    let source = (0..32 * 32)
        .flat_map(|i| {
            [
                (i * 71) as u8,
                (i * 37 + 8) as u8,
                (i * 113 + 12) as u8,
                255,
            ]
        })
        .collect::<Vec<_>>();
    let palette = [[0, 0, 0], [255, 255, 255], [220, 40, 80], [20, 190, 230]]
        .map(|rgb| PaletteEntry::Color { rgb });
    let quantize = QuantizeRequest {
        source_width: 32,
        source_height: 32,
        palette: &palette,
        alpha: AlphaPolicy::Premultiplied {},
        matching: MatchPolicy::SrgbEuclidean,
    };
    let dither = DitherPolicy::Yliluoma {
        size: BayerSize::Two,
        placement: Placement::Everywhere {},
    };
    let run = |limit| {
        let mut processor = Processor::new(limit, 0)?;
        let output =
            processor.dither_and_quantize(quantize, dither, &mut PipelineBoundary(&source))?;
        Ok::<_, prod::contract::failure::Failure>((output, processor.peak_capacity_bytes()))
    };
    let first_optional = |minimum: u64, upper: u64, baseline: u64| {
        let (mut low, mut high) = (minimum, upper);
        while low < high {
            let middle = low + (high - low) / 2;
            if run(middle).unwrap().1 > baseline {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    };

    let (_, roomy_yiluoma_peak) = run(1 << 20).unwrap();
    let yiluoma_minimum = budget_support::minimum(roomy_yiluoma_peak, |limit| run(limit).is_ok());
    let yiluoma_baseline = run(yiluoma_minimum).unwrap().1;
    assert_eq!(yiluoma_baseline, yiluoma_minimum);
    let mut transitions = Vec::new();
    let mut cursor = yiluoma_minimum;
    let mut peak = yiluoma_baseline;
    while transitions.len() < 3 {
        let limit = first_optional(cursor, roomy_yiluoma_peak, peak);
        let next_peak = run(limit).unwrap().1;
        assert!(next_peak > peak);
        transitions.push((limit, next_peak));
        cursor = limit;
        peak = next_peak;
    }
    // Palette retention is the first transition. The index is second and the RGB cache third.
    let index_capacity = transitions[1].1 - transitions[0].1;
    let cache_capacity = transitions[2].1 - transitions[1].1;
    assert!(index_capacity < cache_capacity);

    let tight_limit = yiluoma_minimum + index_capacity + cache_capacity - 1;
    let (actual, tight_peak) = run(tight_limit).unwrap();
    assert_eq!(tight_peak, transitions[1].1);

    let mut reference = request(&source, 32, 32, &palette);
    reference.quantize.alpha = AlphaPolicy::Premultiplied {};
    reference.quantize.matching = MatchPolicy::SrgbEuclidean;
    reference.dither = dither;
    assert_eq!(actual, oracle(reference).indices.data());
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
        let actual = PaletteMatcher::new(
            reference
                .colors
                .iter()
                .map(|entry| PaletteColor::new(entry.index, entry.coordinates))
                .collect(),
            matching,
        );
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
    let matcher = PaletteMatcher::new(
        [0.0, 128.0 / 255.0, 64.0 / 255.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| PaletteColor::new(index as u8, [value; 3]))
            .collect(),
        MatchPolicy::SrgbEuclidean,
    );
    let target = yiluoma::adaptive_target([0.8; 3], matcher.colors()[2].coordinates, 0.0);
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
    let matcher = PaletteMatcher::new(
        vec![
            PaletteColor::new(0, [0.0; 3]),
            PaletteColor::new(1, [1.0; 3]),
            PaletteColor::new(2, [1.0; 3]),
        ],
        MatchPolicy::SrgbEuclidean,
    );
    assert_eq!(
        yiluoma::best_matched_mix([0.375; 3], &matcher, 4),
        yiluoma::PaletteMix {
            low_index: 0,
            high_index: 1,
            high_ratio: 0.25
        }
    );
}

#[path = "support/budget.rs"]
mod budget_support;
