use ditherette_wasm::{
    image::{
        contracts::PaletteEntry, ImageDimensions, ImageFormat, ImageView, ImageViewMut,
        PaletteIndex8, RowStride,
    },
    prod::{contract::request::*, dither::error_diffusion as production},
    spec::{self, dither::error_diffusion as reference},
};
use serde::{de::DeserializeOwned, Serialize};

const KERNELS: [Diffusion; 4] = [
    Diffusion::FloydSteinberg,
    Diffusion::Sierra,
    Diffusion::SierraLite,
    Diffusion::Atkinson,
];
const MODES: [MatchPolicy; 15] = [
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
const BW: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
];

#[cfg(feature = "bench-subjects")]
#[test]
fn native_benchmark_subject_executes_all_diffusion_recipes() {
    use ditherette_wasm::bench_subjects::{
        self, diffusion, reference::ReferenceRequest, BenchSubject,
    };
    let registry = bench_subjects::bench_subjects();
    let subject = |id: &str| {
        registry
            .iter()
            .find_map(|subject| match subject {
                BenchSubject::Conformance(subject) if subject.descriptor.id.as_str() == id => {
                    Some(subject)
                }
                _ => None,
            })
            .unwrap()
    };
    let production = subject(diffusion::SUBJECT);
    let frozen = subject("spec:dither-and-quantize:request:v1");
    assert_eq!(production.operation, frozen.operation);
    let data = [
        100, 100, 100, 255, 190, 41, 23, 128, 12, 211, 18, 0, 43, 71, 111, 255,
    ];
    for kernel in KERNELS {
        for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
            for matching in MODES {
                let input = ReferenceRequest::Processing(
                    spec::contract::request::Request::DitherAndQuantize(
                        spec::contract::request::DitherQuantizeRequest {
                            quantize: spec::contract::request::QuantizeRequest {
                                version: 1,
                                source: spec::contract::request::Source {
                                    width: 2,
                                    height: 2,
                                    data: &data,
                                },
                                palette: &BW,
                                alpha: same_tag(AlphaPolicy::Preserve { threshold: 0.0 }),
                                matching: same_tag(matching),
                            },
                            dither: same_tag(DitherPolicy::Diffusion {
                                kernel,
                                feedback,
                                strength: 0.75,
                                serpentine: true,
                                placement: Placement::Adaptive {
                                    radius: 1,
                                    threshold: 5.0,
                                    softness: 10.0,
                                },
                            }),
                        },
                    ),
                );
                assert_eq!(
                    (production.run)(&input).unwrap(),
                    (frozen.run)(&input).unwrap()
                );
                let direct = diffusion::function(diffusion::SUBJECT).unwrap()(
                    diffusion::request(&input).unwrap(),
                )
                .unwrap();
                assert_eq!(
                    direct,
                    reference::diffuse(match input {
                        ReferenceRequest::Processing(
                            spec::contract::request::Request::DitherAndQuantize(input),
                        ) => input,
                        _ => unreachable!(),
                    })
                    .unwrap()
                );
            }
        }
    }
    assert!(diffusion::function("spec:dither-and-quantize:request:v1").is_none());
}

fn same_tag<T: Serialize, U: DeserializeOwned>(value: T) -> U {
    serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
}

fn request(data: &[u8], width: u32, height: u32) -> DitherQuantizeRequest<'_> {
    DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source: Source {
                width,
                height,
                data,
            },
            palette: &BW,
            alpha: AlphaPolicy::Preserve { threshold: 0.0 },
            matching: MatchPolicy::SrgbEuclidean,
        },
        dither: DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            feedback: DiffusionFeedback::SrgbBytes,
            strength: 1.0,
            serpentine: false,
            placement: Placement::Everywhere {},
        },
    }
}

fn oracle(
    input: DitherQuantizeRequest<'_>,
) -> Result<ditherette_wasm::image::contracts::IndexedImage, spec::contract::error::DitheretteError>
{
    reference::diffuse(spec::contract::request::DitherQuantizeRequest {
        quantize: spec::contract::request::QuantizeRequest {
            version: input.quantize.version,
            source: spec::contract::request::Source {
                width: input.quantize.source.width,
                height: input.quantize.source.height,
                data: input.quantize.source.data,
            },
            palette: input.quantize.palette,
            alpha: same_tag(input.quantize.alpha),
            matching: same_tag(input.quantize.matching),
        },
        dither: same_tag(input.dither),
    })
}

fn compare(input: DitherQuantizeRequest<'_>) {
    match (production::diffuse(input), oracle(input)) {
        (Ok(actual), Ok(expected)) => assert_eq!(actual, expected, "{:?}", input.dither),
        (Err(actual), Err(expected)) => assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        ),
        (actual, expected) => panic!("production {actual:?}, reference {expected:?}"),
    }
}

#[test]
fn complete_diffusion_matches_all_kernels_feedbacks_metrics_scans_alpha_and_placement() {
    let palette = [
        BW[0],
        PaletteEntry::Color { rgb: [181, 31, 91] },
        BW[1],
        BW[1],
        PaletteEntry::Transparent {},
    ];
    let mut cases = 0;
    for (width, height) in [(1, 1), (1, 4), (4, 1), (3, 3)] {
        let data: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 73 + 17) as u8,
                    (i * 31 + 99) as u8,
                    (i * 117 + 41) as u8,
                    [0, 127, 128, 255][i as usize % 4],
                ]
            })
            .collect();
        let original = data.clone();
        for kernel in KERNELS {
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                for matching in MODES {
                    for serpentine in [false, true] {
                        for alpha in [
                            AlphaPolicy::Preserve {
                                threshold: 127.9999999,
                            },
                            AlphaPolicy::Premultiplied {},
                            AlphaPolicy::Matte { rgb: [33, 71, 109] },
                        ] {
                            for placement in [
                                Placement::Everywhere {},
                                Placement::Adaptive {
                                    radius: 1,
                                    threshold: 5.0,
                                    softness: 10.0,
                                },
                                Placement::Adaptive {
                                    radius: 3,
                                    threshold: 100.0,
                                    softness: 0.0,
                                },
                            ] {
                                let mut input = request(&data, width, height);
                                input.quantize.palette = &palette;
                                input.quantize.alpha = alpha;
                                input.quantize.matching = matching;
                                input.dither = DitherPolicy::Diffusion {
                                    kernel,
                                    feedback,
                                    strength: 0.75,
                                    serpentine,
                                    placement,
                                };
                                compare(input);
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(data, original);
    }
    assert_eq!(cases, 8640);
}

#[test]
fn zero_strength_matches_frozen_direct_quantization() {
    let data = [
        100, 100, 100, 255, 200, 80, 31, 128, 13, 231, 77, 0, 255, 255, 255, 255,
    ];
    for matching in MODES {
        for kernel in KERNELS {
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                for serpentine in [false, true] {
                    let mut input = request(&data, 2, 2);
                    input.quantize.matching = matching;
                    input.dither = DitherPolicy::Diffusion {
                        kernel,
                        feedback,
                        strength: 0.0,
                        serpentine,
                        placement: Placement::Adaptive {
                            radius: 1,
                            threshold: 5.0,
                            softness: 10.0,
                        },
                    };
                    let quantized =
                        spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                            version: 1,
                            source: spec::contract::request::Source {
                                width: 2,
                                height: 2,
                                data: &data,
                            },
                            palette: &BW,
                            alpha: same_tag(input.quantize.alpha),
                            matching: same_tag(matching),
                        })
                        .unwrap();
                    assert_eq!(production::diffuse(input).unwrap(), quantized);
                }
            }
        }
    }
}

#[test]
fn first_ties_byte_rounding_and_scan_order_keep_independent_witnesses() {
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [2, 0, 0] },
    ];
    let bytes = [1, 0, 0, 255, 1, 0, 0, 255];
    for (feedback, expected) in [
        (DiffusionFeedback::SrgbBytes, [0, 0]),
        (DiffusionFeedback::Matching, [0, 1]),
    ] {
        let mut input = request(&bytes, 2, 1);
        input.quantize.palette = &palette;
        if let DitherPolicy::Diffusion { feedback: mode, .. } = &mut input.dither {
            *mode = feedback;
        }
        assert_eq!(production::diffuse(input).unwrap().indices.data(), expected);
    }
    let gray: Vec<u8> = (0..4).flat_map(|_| [100, 100, 100, 255]).collect();
    for (kernel, raster, reverse) in [
        (Diffusion::FloydSteinberg, [0, 1, 0, 0], [0, 1, 1, 0]),
        (Diffusion::Sierra, [0, 0, 1, 0], [0, 0, 0, 1]),
        (Diffusion::SierraLite, [0, 1, 0, 0], [0, 1, 1, 0]),
        (Diffusion::Atkinson, [0, 0, 0, 1], [0, 0, 1, 0]),
    ] {
        for (serpentine, expected) in [(false, raster), (true, reverse)] {
            let mut input = request(&gray, 2, 2);
            input.dither = DitherPolicy::Diffusion {
                kernel,
                serpentine,
                feedback: DiffusionFeedback::SrgbBytes,
                strength: 1.0,
                placement: Placement::Everywhere {},
            };
            assert_eq!(production::diffuse(input).unwrap().indices.data(), expected);
        }
    }
}

#[test]
fn sinks_extreme_strength_and_structured_failure_order_match_frozen() {
    for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
        for kernel in KERNELS {
            for data in [
                vec![100, 100, 100, 255, 100, 100, 100, 255],
                vec![255, 255, 255, 255, 255, 255, 255, 255],
                vec![100, 100, 100, 255, 255, 99, 11, 0],
            ] {
                for palette in [
                    vec![BW[0], BW[1], PaletteEntry::Transparent {}],
                    BW.to_vec(),
                    vec![PaletteEntry::Transparent {}; 257],
                ] {
                    let mut input = request(&data, 2, 1);
                    input.quantize.palette = &palette;
                    input.dither = DitherPolicy::Diffusion {
                        kernel,
                        feedback,
                        strength: f32::MAX,
                        serpentine: true,
                        placement: Placement::Everywhere {},
                    };
                    compare(input);
                }
            }
        }
    }
    let data = [100, 100, 100, 255, 100, 100, 100, 255];
    for (feedback, message) in [
        (
            DiffusionFeedback::SrgbBytes,
            "Diffusion work exceeded finite f32 range.",
        ),
        (
            DiffusionFeedback::Matching,
            "Diffusion matching produced a non-finite distance.",
        ),
    ] {
        let mut input = request(&data, 2, 1);
        input.dither = DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            feedback,
            strength: f32::MAX,
            serpentine: false,
            placement: Placement::Everywhere {},
        };
        let failure = production::diffuse(input).unwrap_err();
        assert_eq!(failure.path, "dither.arithmetic");
        assert_eq!(failure.message, message);
    }
    let mut input = request(&data, 2, 1);
    input.dither = DitherPolicy::None {};
    compare(input);
    input.quantize.version = 2;
    compare(input);
    input = request(&data, 2, 1);
    if let DitherPolicy::Diffusion { strength, .. } = &mut input.dither {
        *strength = -1.0;
    }
    compare(input);
}

#[test]
fn reused_palette_and_coordinate_preparation_keep_frozen_values() {
    use ditherette_wasm::prod::{
        color::packed::{Converter, PackedSpace},
        palette::PreparedPalette,
    };
    for palette in [
        vec![BW[0], PaletteEntry::Transparent {}, BW[1], BW[1]],
        BW.to_vec(),
        vec![PaletteEntry::Transparent {}; 257],
        (0..257)
            .map(|i| match i % 17 {
                0 => PaletteEntry::Transparent {},
                1 => PaletteEntry::Color { rgb: [73; 3] },
                _ => PaletteEntry::Color {
                    rgb: [(i * 73) as u8, (i * 117) as u8, (i * 31) as u8],
                },
            })
            .collect(),
    ] {
        for alpha in [
            AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [17, 33, 71] },
        ] {
            let actual = PreparedPalette::try_new(&palette, alpha, u64::MAX).unwrap();
            let expected = spec::palette::PreparedPalette::new(&palette, same_tag(alpha));
            assert_eq!(actual.palette, expected.palette);
            assert_eq!(actual.warnings, expected.warnings);
            assert_eq!(
                actual
                    .visible
                    .iter()
                    .map(|v| (v.index, v.rgb))
                    .collect::<Vec<_>>(),
                expected
                    .visible
                    .iter()
                    .map(|v| (v.index, v.rgb))
                    .collect::<Vec<_>>()
            );
            for matching in MODES {
                let converter = Converter::new(PackedSpace::from_matching(matching).unwrap());
                let expected_matcher =
                    spec::quantize::matcher::PaletteMatcher::new(&expected, same_tag(matching));
                assert_eq!(
                    actual
                        .visible
                        .iter()
                        .map(|entry| (
                            entry.index,
                            converter.coordinates(entry.rgb).map(f32::to_bits)
                        ))
                        .collect::<Vec<_>>(),
                    expected_matcher
                        .colors
                        .iter()
                        .map(|entry| (entry.index, entry.coordinates.map(f32::to_bits)))
                        .collect::<Vec<_>>()
                );
            }
            for rgba in [
                [91, 173, 39, 0],
                [91, 173, 39, 127],
                [91, 173, 39, 128],
                [91, 173, 39, 255],
            ] {
                assert_eq!(
                    format!("{:?}", actual.prepare_pixel(rgba)),
                    format!("{:?}", expected.prepare_pixel(rgba))
                );
            }
        }
    }
}

fn coordinate_rows<F: ImageFormat<Storage = f32> + Copy>() {
    let dimensions = ImageDimensions::new(3, 3).unwrap();
    let stride = 3 * F::CHANNEL_COUNT + 2;
    let mut data = vec![73.0; stride * 3];
    for y in 0..3 {
        for x in 0..3 {
            for c in 0..3 {
                data[y * stride + x * F::CHANNEL_COUNT + c] = (y * 3 + x + c) as f32 / 11.0;
            }
        }
    }
    let original = data.clone();
    let source = ImageView::<F>::new(&data, dimensions, RowStride::new(stride).unwrap()).unwrap();
    let palette = [[0.0; 3], [1.0; 3], [1.0; 3]];
    for (actual_kernel, expected_kernel) in [
        (
            production::ErrorDiffusionKernel::FloydSteinberg,
            reference::ErrorDiffusionKernel::FloydSteinberg,
        ),
        (
            production::ErrorDiffusionKernel::Sierra,
            reference::ErrorDiffusionKernel::Sierra,
        ),
        (
            production::ErrorDiffusionKernel::SierraLite,
            reference::ErrorDiffusionKernel::SierraLite,
        ),
        (
            production::ErrorDiffusionKernel::Atkinson,
            reference::ErrorDiffusionKernel::Atkinson,
        ),
    ] {
        assert_eq!(
            actual_kernel
                .taps()
                .iter()
                .map(|t| (t.dx, t.dy, t.weight.to_bits()))
                .collect::<Vec<_>>(),
            expected_kernel
                .taps()
                .iter()
                .map(|t| (t.dx, t.dy, t.weight.to_bits()))
                .collect::<Vec<_>>()
        );
        for serpentine in [false, true] {
            let mut actual = vec![199; 15];
            let mut expected = actual.clone();
            production::dither_error_diffusion_into(
                source,
                &palette,
                ImageViewMut::<PaletteIndex8>::new(
                    &mut actual,
                    dimensions,
                    RowStride::new(5).unwrap(),
                )
                .unwrap(),
                actual_kernel,
                0.75,
                serpentine,
            );
            reference::dither_error_diffusion_into(
                source,
                &palette,
                ImageViewMut::<PaletteIndex8>::new(
                    &mut expected,
                    dimensions,
                    RowStride::new(5).unwrap(),
                )
                .unwrap(),
                expected_kernel,
                0.75,
                serpentine,
            );
            assert_eq!(actual, expected);
            for row in actual.chunks_exact(5) {
                assert_eq!(&row[3..], &[199, 199]);
            }
        }
    }
    assert_eq!(data, original);
}

#[test]
fn legacy_coordinate_exports_preserve_taps_strides_padding_and_three_or_four_channels() {
    coordinate_rows::<ditherette_wasm::image::Srgb32>();
    coordinate_rows::<ditherette_wasm::image::LinearRgba32>();
}
