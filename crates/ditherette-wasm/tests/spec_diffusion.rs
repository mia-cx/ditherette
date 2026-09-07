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
            placement: Placement::Everywhere,
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
