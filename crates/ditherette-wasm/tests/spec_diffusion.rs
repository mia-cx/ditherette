use ditherette_wasm::spec::dither::error_diffusion::{
    ErrorDiffusionKernel, ATKINSON_TAPS, FLOYD_STEINBERG_TAPS, SIERRA_LITE_TAPS, SIERRA_TAPS,
};
use ditherette_wasm::{
    image::contracts::{PaletteEntry, WarningCode},
    spec::{
        contract::{
            error::ErrorCode,
            request::{
                AlphaPolicy, Diffusion, DiffusionFeedback, DitherPolicy, DitherQuantizeRequest,
                MatchPolicy, Placement, QuantizeRequest, Source,
            },
        },
        dither::error_diffusion::diffuse,
        quantize::quantize,
    },
};

const BW: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
];
const KERNELS: [Diffusion; 4] = [
    Diffusion::FloydSteinberg,
    Diffusion::Sierra,
    Diffusion::SierraLite,
    Diffusion::Atkinson,
];
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

fn grays(values: &[u8]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&value| [value, value, value, 255])
        .collect()
}

fn request(
    source: &[u8],
    width: u32,
    height: u32,
    kernel: Diffusion,
    feedback: DiffusionFeedback,
) -> DitherQuantizeRequest<'_> {
    DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source: Source {
                width,
                height,
                data: source,
            },
            palette: &BW,
            alpha: AlphaPolicy::Preserve { threshold: 0.0 },
            matching: MatchPolicy::SrgbEuclidean,
        },
        dither: DitherPolicy::Diffusion {
            kernel,
            strength: 1.0,
            placement: Placement::Everywhere {},
            serpentine: false,
            feedback,
        },
    }
}

#[test]
fn tap_positions_weights_and_intentional_atkinson_loss_are_exact() {
    // Numerators and denominators make normalization independent of the production constants.
    let cases: &[(ErrorDiffusionKernel, &[(i32, u32, u32)], u32, f32)] = &[
        (
            ErrorDiffusionKernel::FloydSteinberg,
            &[(1, 0, 7), (-1, 1, 3), (0, 1, 5), (1, 1, 1)],
            16,
            1.0,
        ),
        (
            ErrorDiffusionKernel::Sierra,
            &[
                (1, 0, 5),
                (2, 0, 3),
                (-2, 1, 2),
                (-1, 1, 4),
                (0, 1, 5),
                (1, 1, 4),
                (2, 1, 2),
                (-1, 2, 2),
                (0, 2, 3),
                (1, 2, 2),
            ],
            32,
            1.0,
        ),
        (
            ErrorDiffusionKernel::SierraLite,
            &[(1, 0, 2), (-1, 1, 1), (0, 1, 1)],
            4,
            1.0,
        ),
        (
            ErrorDiffusionKernel::Atkinson,
            &[
                (1, 0, 1),
                (2, 0, 1),
                (-1, 1, 1),
                (0, 1, 1),
                (1, 1, 1),
                (0, 2, 1),
            ],
            8,
            0.75,
        ),
    ];
    for (kernel, positions, denominator, sum) in cases {
        let taps = kernel.taps();
        assert_eq!(taps.len(), positions.len());
        for (tap, &(dx, dy, numerator)) in taps.iter().zip(*positions) {
            assert_eq!((tap.dx, tap.dy), (dx, dy));
            assert_eq!(tap.weight, numerator as f32 / *denominator as f32);
            assert!(dy > 0 || dx > 0, "every tap follows the raster scan");
        }
        assert_eq!(taps.iter().map(|tap| tap.weight).sum::<f32>(), *sum);
    }
    assert_eq!(
        ErrorDiffusionKernel::FloydSteinberg.taps(),
        FLOYD_STEINBERG_TAPS
    );
    assert_eq!(ErrorDiffusionKernel::Sierra.taps(), SIERRA_TAPS);
    assert_eq!(ErrorDiffusionKernel::SierraLite.taps(), SIERRA_LITE_TAPS);
    assert_eq!(ErrorDiffusionKernel::Atkinson.taps(), ATKINSON_TAPS);
}

#[test]
fn byte_feedback_and_matching_srgb_have_distinct_two_pixel_results() {
    let source = [1, 0, 0, 255, 1, 0, 0, 255];
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [2, 0, 0] },
    ];
    for (feedback, expected) in [
        (DiffusionFeedback::SrgbBytes, [0, 0]),
        (DiffusionFeedback::Matching, [0, 1]),
    ] {
        let mut input = request(&source, 2, 1, Diffusion::FloydSteinberg, feedback);
        input.quantize.palette = &palette;
        // First red=1 ties at index 0. The next red=1+7/16 is rounded only by byte feedback.
        assert_eq!(diffuse(input).unwrap().indices.data(), expected);
    }
}

#[test]
fn transparent_pixels_drop_incoming_error_and_never_emit_hidden_rgb() {
    for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
        for hidden in [0, 255] {
            let source = [
                100, 100, 100, 255, hidden, hidden, hidden, 0, 100, 100, 100, 255,
            ];
            let palette = [BW[0], BW[1], PaletteEntry::Transparent {}];
            let mut input = request(&source, 3, 1, Diffusion::FloydSteinberg, feedback);
            input.quantize.palette = &palette;
            let result = diffuse(input).unwrap();
            assert_eq!(result.indices.data(), [0, 2, 0]);
            assert!(result.warnings.is_empty());
            input.quantize.palette = &BW;
            let fallback = diffuse(input).unwrap();
            assert_eq!(fallback.indices.data(), [0, 0, 0]);
            assert_eq!(fallback.warnings[0].code, WarningCode::TransparentFallback);
        }
    }
}

#[test]
fn work_overflow_and_distance_overflow_are_separate_structured_failures() {
    let source = grays(&[100, 100]);
    let original = source.clone();
    let stable = diffuse(request(
        &source,
        2,
        1,
        Diffusion::FloydSteinberg,
        DiffusionFeedback::SrgbBytes,
    ))
    .unwrap();
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
        let mut input = request(&source, 2, 1, Diffusion::FloydSteinberg, feedback);
        if let DitherPolicy::Diffusion { strength, .. } = &mut input.dither {
            *strength = f32::MAX;
        }
        let error = diffuse(input).unwrap_err();
        assert_eq!(
            (error.code, error.path.as_str()),
            (ErrorCode::Runtime, "dither.arithmetic")
        );
        assert_eq!(error.message, message);
    }
    assert_eq!(source, original);
    assert_eq!(stable.indices.data(), [0, 1]);
}

#[test]
fn extreme_strength_is_valid_when_no_useful_error_overflows() {
    for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
        for source in [grays(&[255, 255]), vec![100, 100, 100, 255, 0, 0, 0, 0]] {
            let palette = [BW[0], BW[1], PaletteEntry::Transparent {}];
            let mut input = request(&source, 2, 1, Diffusion::FloydSteinberg, feedback);
            input.quantize.palette = &palette;
            if let DitherPolicy::Diffusion { strength, .. } = &mut input.dither {
                *strength = f32::MAX;
            }
            let expected = if source[0] == 255 { [1, 1] } else { [0, 2] };
            assert_eq!(diffuse(input).unwrap().indices.data(), expected);
        }
    }
}

#[test]
fn zero_strength_equals_direct_quantization_for_every_kernel_feedback_and_metric() {
    let source = [
        100, 100, 100, 255, 255, 255, 255, 128, 255, 0, 0, 0, 0, 255, 0, 255, 0, 0, 0, 255, 0, 0,
        255, 255,
    ];
    let palette = [
        BW[0],
        PaletteEntry::Transparent {},
        BW[1],
        PaletteEntry::Color { rgb: [0, 255, 0] },
        PaletteEntry::Color { rgb: [0, 0, 255] },
    ];
    for matching in POLICIES {
        for kernel in KERNELS {
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                for serpentine in [false, true] {
                    let mut input = request(&source, 3, 2, kernel, feedback);
                    input.quantize.palette = &palette;
                    input.quantize.matching = matching;
                    input.quantize.alpha = AlphaPolicy::Preserve { threshold: 128.0 };
                    input.dither = DitherPolicy::Diffusion {
                        kernel,
                        feedback,
                        strength: 0.0,
                        serpentine,
                        placement: Placement::Adaptive {
                            radius: 1,
                            threshold: 20.0,
                            softness: 10.0,
                        },
                    };
                    assert_eq!(
                        diffuse(input).unwrap(),
                        quantize(input.quantize).unwrap(),
                        "{kernel:?} {matching:?} {feedback:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn two_by_two_hand_calculation_distinguishes_raster_and_serpentine_scans() {
    let source = grays(&[100; 4]);
    for (kernel, raster, serpentine) in [
        (Diffusion::FloydSteinberg, [0, 1, 0, 0], [0, 1, 1, 0]),
        (Diffusion::Sierra, [0, 0, 1, 0], [0, 0, 0, 1]),
        (Diffusion::SierraLite, [0, 1, 0, 0], [0, 1, 1, 0]),
        (Diffusion::Atkinson, [0, 0, 0, 1], [0, 0, 1, 0]),
    ] {
        for (reverse, expected) in [(false, raster), (true, serpentine)] {
            let mut input = request(&source, 2, 2, kernel, DiffusionFeedback::SrgbBytes);
            if let DitherPolicy::Diffusion { serpentine, .. } = &mut input.dither {
                *serpentine = reverse;
            }
            // Floyd starts the last row at [110.4375,71.5625]. Reversal changes which residual arrives first.
            assert_eq!(
                diffuse(input).unwrap().indices.data(),
                expected,
                "{kernel:?} reverse={reverse}"
            );
        }
    }
}

#[test]
fn one_column_exposes_two_row_taps_without_edge_renormalization() {
    let source = grays(&[64, 0, 120]);
    for (kernel, expected) in [
        (Diffusion::FloydSteinberg, [0, 0, 0]),
        (Diffusion::Sierra, [0, 0, 1]),
        (Diffusion::SierraLite, [0, 0, 0]),
        (Diffusion::Atkinson, [0, 0, 1]),
    ] {
        // Last unrounded byte is 126.25 / 127.5625 / 124 / 129, respectively.
        assert_eq!(
            diffuse(request(&source, 1, 3, kernel, DiffusionFeedback::SrgbBytes))
                .unwrap()
                .indices
                .data(),
            expected,
            "{kernel:?}"
        );
        let single = grays(&[64]);
        assert_eq!(
            diffuse(request(&single, 1, 1, kernel, DiffusionFeedback::SrgbBytes))
                .unwrap()
                .indices
                .data(),
            [0]
        );
    }
}

#[test]
fn positive_strength_uses_every_metric_and_feedback_coordinate_system() {
    let source = grays(&[100, 100]);
    for matching in POLICIES {
        let expected = match matching {
            MatchPolicy::LinearRgbEuclidean => [0, 0],
            MatchPolicy::OklabEuclidean
            | MatchPolicy::OklchEuclidean
            | MatchPolicy::OklchCircularHue
            | MatchPolicy::OklchHueArc => [1, 0],
            _ => [0, 1],
        };
        for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
            let mut input = request(&source, 2, 1, Diffusion::FloydSteinberg, feedback);
            input.quantize.matching = matching;
            // Gray100 has linear light≈.1274, Oklab L≈.5032, CIELAB L≈42.375, and encoded Y≈.3922.
            assert_eq!(
                diffuse(input).unwrap().indices.data(),
                expected,
                "{matching:?} {feedback:?}"
            );
        }
    }
}

#[test]
fn placement_reads_unchanged_source_and_scales_only_outgoing_error() {
    for (gray, expected) in [(80, [0, 0, 0, 0]), (100, [0, 0, 1, 0])] {
        let source = grays(&[0, gray, gray, gray]);
        let mut input = request(
            &source,
            4,
            1,
            Diffusion::FloydSteinberg,
            DiffusionFeedback::SrgbBytes,
        );
        if let DitherPolicy::Diffusion { placement, .. } = &mut input.dither {
            *placement = Placement::Adaptive {
                radius: 1,
                threshold: 5.0,
                softness: 0.0,
            };
        }
        // Source contrast at x=2 is zero despite incoming error. At gray100 that pixel still matches white.
        assert_eq!(diffuse(input).unwrap().indices.data(), expected);
    }
    let source = grays(&[0, 80, 80, 80]);
    assert_eq!(
        diffuse(request(
            &source,
            4,
            1,
            Diffusion::FloydSteinberg,
            DiffusionFeedback::SrgbBytes
        ))
        .unwrap()
        .indices
        .data(),
        [0, 0, 0, 1]
    );
}

#[test]
fn byte_feedback_placement_uses_matching_space_and_includes_hidden_rgb() {
    let palette = [
        BW[0],
        PaletteEntry::Color { rgb: [100; 3] },
        PaletteEntry::Transparent {},
    ];
    for (hidden, expected) in [(0, [2, 1, 1]), (255, [2, 1, 0])] {
        let source = [hidden, hidden, hidden, 0, 80, 80, 80, 255, 80, 80, 80, 255];
        let mut input = request(
            &source,
            3,
            1,
            Diffusion::FloydSteinberg,
            DiffusionFeedback::SrgbBytes,
        );
        input.quantize.palette = &palette;
        input.quantize.matching = MatchPolicy::LinearRgbEuclidean;
        if let DitherPolicy::Diffusion { placement, .. } = &mut input.dither {
            *placement = Placement::Adaptive {
                radius: 1,
                threshold: 5.0,
                softness: 0.0,
            };
        }
        // Hidden black gives linear contrast≈3%, not encoded contrast≈11.76%; hidden white gives≈34.49%.
        assert_eq!(diffuse(input).unwrap().indices.data(), expected);
    }
}

#[test]
fn compositing_and_transparent_only_metadata_survive_the_complete_call() {
    let source = [200, 100, 50, 128];
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [100, 50, 25] },
        PaletteEntry::Color {
            rgb: [100, 50, 152],
        },
    ];
    for (alpha, expected) in [
        (AlphaPolicy::Premultiplied {}, [1]),
        (AlphaPolicy::Matte { rgb: [0, 0, 255] }, [2]),
    ] {
        for kernel in KERNELS {
            for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
                let mut input = request(&source, 1, 1, kernel, feedback);
                input.quantize.palette = &palette;
                input.quantize.alpha = alpha;
                assert_eq!(diffuse(input).unwrap().indices.data(), expected);
            }
        }
    }
    let palette = vec![PaletteEntry::Transparent {}; 257];
    let mut input = request(
        &source,
        1,
        1,
        Diffusion::Atkinson,
        DiffusionFeedback::Matching,
    );
    input.quantize.palette = &palette;
    let result = diffuse(input).unwrap();
    assert_eq!(result.indices.data(), [0]);
    assert_eq!(result.palette.transparent_index, Some(0));
    assert_eq!(result.palette.rgba, vec![0; 1024]);
    assert_eq!(
        result
            .warnings
            .iter()
            .map(|warning| warning.code)
            .collect::<Vec<_>>(),
        [WarningCode::PaletteTruncated, WarningCode::TransparentOnly]
    );
}

#[test]
fn complete_diffusion_rejects_wrong_families_and_invalid_settings_without_mutation() {
    let source = grays(&[100]);
    let original = source.clone();
    let mut input = request(
        &source,
        1,
        1,
        Diffusion::FloydSteinberg,
        DiffusionFeedback::SrgbBytes,
    );
    input.dither = DitherPolicy::None {};
    let error = diffuse(input).unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::UnsupportedOperation, "dither.family")
    );
    input = request(
        &source,
        1,
        1,
        Diffusion::FloydSteinberg,
        DiffusionFeedback::SrgbBytes,
    );
    if let DitherPolicy::Diffusion { strength, .. } = &mut input.dither {
        *strength = -1.0;
    }
    let error = diffuse(input).unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidSettings, "dither.strength")
    );
    input.quantize.version = 2;
    assert_eq!(diffuse(input).unwrap_err().path, "version");
    assert_eq!(source, original);
}
