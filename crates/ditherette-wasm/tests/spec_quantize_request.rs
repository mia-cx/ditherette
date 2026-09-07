use ditherette_wasm::{
    image::contracts::{PaletteEntry, WarningCode},
    spec::{
        contract::{
            error::ErrorCode,
            request::{parse_match, AlphaPolicy, MatchPolicy, QuantizeRequest, Source},
        },
        quantize::{
            matcher::{PaletteColor, PaletteMatcher},
            quantize,
        },
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
const PRESERVE: AlphaPolicy = AlphaPolicy::Preserve { threshold: 0.0 };

fn request<'a>(
    data: &'a [u8],
    palette: &'a [PaletteEntry],
    matching: MatchPolicy,
) -> QuantizeRequest<'a> {
    QuantizeRequest {
        version: 1,
        source: Source {
            width: (data.len() / 4) as u32,
            height: 1,
            data,
        },
        palette,
        alpha: PRESERVE,
        matching,
    }
}

#[test]
fn every_pair_quantizes_exact_colors_and_preserves_original_duplicate_indices() {
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let source = [
        255, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 1,
    ];
    for matching in POLICIES {
        let result = quantize(request(&source, &palette, matching)).unwrap();
        assert_eq!(result.indices.data(), [2, 1, 0, 4, 2], "{matching:?}");
        assert_eq!(result.indices.dimensions().width(), 5);
        assert_eq!(result.indices.dimensions().height(), 1);
        assert_eq!(
            result.palette.rgba,
            [0, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255]
        );
        assert_eq!(result.palette.transparent_index, Some(1));
        assert!(result.warnings.is_empty());
    }
}

#[test]
fn midpoint_ties_keep_the_first_visible_entry_even_after_transparency() {
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [2, 0, 0] },
    ];
    let source = [1, 0, 0, 255];
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
    ] {
        assert_eq!(
            quantize(request(&source, &palette, matching))
                .unwrap()
                .indices
                .data(),
            [1]
        );
    }
}

#[test]
fn linear_matching_uses_linear_light_not_encoded_midpoints() {
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let source = [128, 128, 128, 255];
    // Byte 128 has linear light about .216, Oklab L about .600, and CIELAB L about 53.585.
    // Neutral hue is zero. Every other recipe is nearer white; linear RGB is nearer black.
    for matching in POLICIES {
        let expected = if matching == MatchPolicy::LinearRgbEuclidean {
            0
        } else {
            1
        };
        assert_eq!(
            quantize(request(&source, &palette, matching))
                .unwrap()
                .indices
                .data(),
            [expected],
            "{matching:?}"
        );
    }
}

#[test]
fn weighted_rgb_complete_calls_follow_hand_calculated_winners() {
    let source = [0, 0, 0, 255];
    let palette = [
        PaletteEntry::Color { rgb: [17, 0, 0] },
        PaletteEntry::Color { rgb: [0, 0, 14] },
    ];
    // Byte-squared CompuPhase scores are 587.595703125 and 587.234375.
    // The inherited missing 255/256 factors incorrectly select red instead.
    assert_eq!(
        quantize(request(&source, &palette, MatchPolicy::SrgbCompuphase))
            .unwrap()
            .indices
            .data(),
        [1]
    );
    let palette = [
        PaletteEntry::Color { rgb: [15, 0, 0] },
        PaletteEntry::Color { rgb: [0, 0, 25] },
    ];
    // Rec.601 scores 67.275 / 71.25; Rec.709 scores 47.835 / 45.125.
    for (matching, expected) in [(MatchPolicy::SrgbRec601, 0), (MatchPolicy::SrgbRec709, 1)] {
        assert_eq!(
            quantize(request(&source, &palette, matching))
                .unwrap()
                .indices
                .data(),
            [expected]
        );
    }
}

#[test]
fn unequal_chroma_distinguishes_the_chord_and_website_arc_recipes() {
    let colors = vec![
        PaletteColor {
            index: 3,
            coordinates: [0.5, 0.2, std::f32::consts::FRAC_PI_2],
        },
        PaletteColor {
            index: 7,
            coordinates: [0.5, 0.3, 0.0],
        },
    ];
    // For source C=.1, candidate scores are .05 / .04 (chord), .03467401 / .04 (arc).
    for (matching, expected) in [
        (MatchPolicy::OklchCircularHue, 7),
        (MatchPolicy::CielchCircularHue, 7),
        (MatchPolicy::OklchHueArc, 3),
        (MatchPolicy::CielchHueArc, 3),
    ] {
        let matcher = PaletteMatcher {
            colors: colors.clone(),
            matching,
        };
        assert_eq!(matcher.nearest([0.5, 0.1, 0.0]).index, expected);
    }
}

#[test]
fn alpha_preparation_precedes_color_conversion_and_matching() {
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color {
            rgb: [200, 100, 50],
        },
        PaletteEntry::Color { rgb: [100, 50, 25] },
        PaletteEntry::Color {
            rgb: [100, 50, 152],
        },
        PaletteEntry::Color { rgb: [0, 0, 255] },
    ];
    let source = [200, 100, 50, 128, 200, 100, 50, 0];
    for (alpha, expected) in [
        (
            AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            [1, 0],
        ),
        (AlphaPolicy::Preserve { threshold: 128.0 }, [0, 0]),
        (AlphaPolicy::Matte { rgb: [0, 0, 255] }, [3, 4]),
    ] {
        for matching in POLICIES {
            let result = quantize(QuantizeRequest {
                alpha,
                ..request(&source, &palette, matching)
            })
            .unwrap();
            assert_eq!(result.indices.data(), expected, "{matching:?} {alpha:?}");
        }
    }
    for matching in POLICIES {
        let result = quantize(QuantizeRequest {
            alpha: AlphaPolicy::Premultiplied {},
            ..request(&source[..4], &palette, matching)
        })
        .unwrap();
        assert_eq!(result.indices.data(), [2], "{matching:?}");
    }
}

#[test]
fn transparent_only_and_darkest_fallback_return_complete_indexed_images() {
    let source = [255, 255, 255, 0, 0, 254, 0, 255];
    for alpha in [
        PRESERVE,
        AlphaPolicy::Premultiplied {},
        AlphaPolicy::Matte { rgb: [255; 3] },
    ] {
        let palette = [PaletteEntry::Transparent {}, PaletteEntry::Transparent {}];
        let result = quantize(QuantizeRequest {
            alpha,
            ..request(&source, &palette, MatchPolicy::CielabCiede2000)
        })
        .unwrap();
        assert_eq!(result.indices.data(), [0, 0]);
        assert_eq!(result.palette.rgba, [0; 8]);
        assert_eq!(result.warnings[0].code, WarningCode::TransparentOnly);
    }
    let palette = [
        PaletteEntry::Color { rgb: [255, 0, 0] },
        PaletteEntry::Color { rgb: [0, 254, 0] },
        PaletteEntry::Color { rgb: [0, 0, 254] },
    ];
    let result = quantize(request(&source, &palette, MatchPolicy::SrgbEuclidean)).unwrap();
    assert_eq!(result.indices.data(), [1, 1]);
    assert_eq!(result.palette.transparent_index, None);
    assert_eq!(result.warnings[0].code, WarningCode::TransparentFallback);
}

#[test]
fn visible_index_255_survives_matching_and_truncation_precedes_alpha_fallback() {
    let mut palette = vec![PaletteEntry::Color { rgb: [0; 3] }; 255];
    palette.push(PaletteEntry::Color { rgb: [255, 0, 0] });
    palette.push(PaletteEntry::Transparent {});
    let source = [255, 0, 0, 255, 255, 0, 0, 0];
    let result = quantize(request(&source, &palette, MatchPolicy::SrgbEuclidean)).unwrap();
    assert_eq!(result.indices.data(), [255, 0]);
    assert_eq!(result.palette.rgba.len(), 1024);
    assert_eq!(result.palette.transparent_index, None);
    assert_eq!(
        result
            .warnings
            .iter()
            .map(|warning| warning.code)
            .collect::<Vec<_>>(),
        [
            WarningCode::PaletteTruncated,
            WarningCode::TransparentFallback
        ]
    );
}

#[test]
fn invalid_requests_return_stable_errors_before_output_preparation() {
    let palette = [PaletteEntry::Color { rgb: [0; 3] }];
    let source = [12, 34, 56, 255];
    let valid = request(&source, &palette, MatchPolicy::SrgbEuclidean);
    for (invalid, code, path) in [
        (
            QuantizeRequest {
                version: 2,
                ..valid
            },
            ErrorCode::InvalidRequest,
            "version",
        ),
        (
            QuantizeRequest {
                palette: &[],
                ..valid
            },
            ErrorCode::InvalidPalette,
            "palette",
        ),
        (
            QuantizeRequest {
                alpha: AlphaPolicy::Preserve {
                    threshold: f64::NAN,
                },
                ..valid
            },
            ErrorCode::InvalidSettings,
            "alpha.threshold",
        ),
        (
            QuantizeRequest {
                source: Source {
                    width: 8192,
                    height: 8192,
                    data: &source,
                },
                ..valid
            },
            ErrorCode::InvalidImage,
            "source.data",
        ),
    ] {
        let error = quantize(invalid).unwrap_err();
        assert_eq!((error.code, error.path.as_str()), (code, path));
    }
    for tag in ["srgb-ciede2000", "linear-rgb-rec601", "cielab-hue-arc"] {
        let error = parse_match(tag, "match").unwrap_err();
        assert_eq!(
            (error.code, error.path.as_str()),
            (ErrorCode::InvalidSettings, "match")
        );
    }
    assert_eq!(source, [12, 34, 56, 255]);
}

#[test]
fn caller_storage_and_prior_outputs_remain_independent_across_calls() {
    let mut source = [
        255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];
    let original_source = source;
    let mut palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let original_palette = palette;
    let mut input = request(&source, &palette, MatchPolicy::SrgbEuclidean);
    input.source.width = 2;
    input.source.height = 2;
    let result = quantize(input).unwrap();
    assert_eq!(source, original_source);
    assert_eq!(palette, original_palette);
    assert_eq!(result.indices.data(), [1, 0, 0, 1]);
    assert_eq!(
        (
            result.indices.dimensions().width(),
            result.indices.dimensions().height()
        ),
        (2, 2)
    );
    source.fill(0);
    palette.reverse();
    let later = quantize(request(&source, &palette, MatchPolicy::SrgbEuclidean)).unwrap();
    assert_eq!(later.indices.data(), [1; 4]);
    assert_eq!(result.indices.data(), [1, 0, 0, 1]);
    assert_eq!(result.palette.rgba, [0, 0, 0, 255, 255, 255, 255, 255]);
}
