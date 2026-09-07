use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, NormalizedPalette, PaletteEntry, ProcessWarning, WarningCode},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    spec::contract::{error::ErrorCode, request::*},
};

const PIXEL: [u8; 4] = [20, 40, 60, 128];
const PALETTE: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Transparent {},
];

fn source() -> Source<'static> {
    Source {
        width: 1,
        height: 1,
        data: &PIXEL,
    }
}
fn output() -> Output {
    Output {
        width: 1,
        height: 1,
        resize: ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
    }
}
fn quantize() -> QuantizeRequest<'static> {
    QuantizeRequest {
        version: 1,
        source: source(),
        palette: &PALETTE,
        alpha: AlphaPolicy::Preserve { threshold: 0.0 },
        matching: MatchPolicy::OklabEuclidean,
    }
}
fn perturb() -> PerturbPolicy {
    PerturbPolicy {
        field: Field::Bayer {
            size: BayerSize::Four,
        },
        space: WorkingSpace::Srgb,
        strength: 1.0,
        placement: Placement::Everywhere,
    }
}
fn recipe() -> RecipeV1 {
    RecipeV1 {
        version: 1,
        output: output(),
        alpha: quantize().alpha,
        matching: quantize().matching,
        dither: DitherPolicy::None,
    }
}

#[test]
fn all_five_requests_borrow_valid_storage_without_changing_it() {
    let requests = [
        Request::Process(ProcessRequest {
            source: source(),
            palette: &PALETTE,
            recipe: recipe(),
        }),
        Request::Resize(ResizeRequest {
            version: 1,
            source: source(),
            output: output(),
        }),
        Request::Perturb(PerturbRequest {
            version: 1,
            source: source(),
            perturb: perturb(),
        }),
        Request::Quantize(quantize()),
        Request::DitherAndQuantize(DitherQuantizeRequest {
            quantize: quantize(),
            dither: DitherPolicy::Diffusion {
                kernel: Diffusion::Atkinson,
                strength: 1.0,
                placement: Placement::Everywhere,
                serpentine: true,
                feedback: DiffusionFeedback::SrgbBytes,
            },
        }),
    ];
    for request in requests {
        let layout = request.validate().unwrap();
        assert_eq!(layout.output.pixel_count().unwrap(), 1);
        assert_eq!(layout.source.data().as_ptr(), PIXEL.as_ptr());
        assert_eq!(layout.source.data(), [20, 40, 60, 128]);
    }
}

#[test]
fn rejects_version_image_length_and_empty_palette_with_stable_paths() {
    let mut request = quantize();
    request.version = 2;
    let error = Request::Quantize(request).validate().unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidRequest, "version")
    );
    request = quantize();
    request.source.data = &PIXEL[..3];
    let error = Request::Quantize(request).validate().unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidImage, "source.data")
    );
    request = quantize();
    request.palette = &[];
    let error = Request::Quantize(request).validate().unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidPalette, "palette")
    );
}

#[test]
fn validates_dimension_limits_without_allocating_large_images() {
    assert!(validate_dimensions(
        32_768,
        2_048,
        MAX_SOURCE_SIDE,
        "source",
        ErrorCode::InvalidImage
    )
    .is_ok());
    assert!(validate_dimensions(
        16_384,
        4_096,
        MAX_OUTPUT_SIDE,
        "output",
        ErrorCode::InvalidSettings
    )
    .is_ok());
    for (width, height, path) in [
        (0, 1, "source.width"),
        (1, 0, "source.height"),
        (32_769, 1, "source.width"),
        (32_768, 2_049, "source"),
    ] {
        let error = validate_dimensions(
            width,
            height,
            MAX_SOURCE_SIDE,
            "source",
            ErrorCode::InvalidImage,
        )
        .unwrap_err();
        assert_eq!(error.path, path);
    }
}

#[test]
fn accepts_transparent_only_and_oversize_palettes_for_later_normalization() {
    let transparent = [PaletteEntry::Transparent {}];
    let oversized = [PaletteEntry::Transparent {}; 257];
    for palette in [&transparent[..], &oversized[..]] {
        let request = QuantizeRequest {
            palette,
            ..quantize()
        };
        assert!(Request::Quantize(request).validate().is_ok());
    }
}

#[test]
fn match_tags_cover_all_coherent_pairs_and_reject_invalid_pairs() {
    let tags = [
        "srgb-euclidean",
        "srgb-compuphase",
        "srgb-rec601",
        "srgb-rec709",
        "linear-rgb-euclidean",
        "oklab-euclidean",
        "oklch-euclidean",
        "oklch-circular-hue",
        "oklch-hue-arc",
        "cielab-euclidean",
        "cielab-ciede2000",
        "cielch-euclidean",
        "cielch-circular-hue",
        "cielch-hue-arc",
        "ycbcr-euclidean",
    ];
    for tag in tags {
        assert!(parse_match(tag, "recipe.match").is_ok(), "{tag}");
    }
    for tag in [
        "oklab-ciede2000",
        "srgb-circular-hue",
        "srgb-hue-arc",
        "linear-rgb-rec709",
        "cielch-ciede2000",
    ] {
        let error = parse_match(tag, "recipe.match").unwrap_err();
        assert_eq!(
            (error.code, error.path.as_str()),
            (ErrorCode::InvalidSettings, "recipe.match")
        );
    }
}

#[test]
fn diffusion_feedback_tags_distinguish_bytes_from_matching_coordinates() {
    for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
        let policy = DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            strength: 1.0,
            placement: Placement::Everywhere,
            serpentine: false,
            feedback,
        };
        let value = serde_json::to_value(policy).unwrap();
        let tag = if feedback == DiffusionFeedback::SrgbBytes {
            "srgb-bytes"
        } else {
            "matching"
        };
        assert_eq!(value["feedback"], tag);
        assert!(value.get("space").is_none());
        assert_eq!(
            serde_json::from_value::<DitherPolicy>(value.clone()).unwrap(),
            policy
        );
        let mut invalid = value;
        invalid["space"] = "srgb".into();
        assert!(serde_json::from_value::<DitherPolicy>(invalid).is_err());
        assert!(Request::DitherAndQuantize(DitherQuantizeRequest {
            quantize: quantize(),
            dither: policy
        })
        .validate()
        .is_ok());
    }
    assert!(serde_json::from_str::<DiffusionFeedback>("\"srgb\"").is_err());
}

#[test]
fn malformed_tags_and_unknown_fields_fail_recipe_decoding() {
    let encoded = serde_json::to_string(&recipe()).unwrap();
    assert_eq!(decode_recipe(&encoded).unwrap(), recipe());
    let mut value = serde_json::to_value(recipe()).unwrap();
    value["match"] = "oklab-ciede2000".into();
    assert_eq!(
        decode_recipe(&value.to_string()).unwrap_err().code,
        ErrorCode::InvalidSettings
    );
    value = serde_json::to_value(recipe()).unwrap();
    value["threadCount"] = 4.into();
    assert_eq!(
        decode_recipe(&value.to_string()).unwrap_err().path,
        "recipe"
    );
    value = serde_json::to_value(recipe()).unwrap();
    value["output"]["width"] = 1.5.into();
    assert!(decode_recipe(&value.to_string()).is_err());
    assert!(serde_json::from_str::<Field>(r#"{"algorithm":"floyd-steinberg"}"#).is_err());
    assert!(serde_json::from_str::<PaletteEntry>(r#"{"kind":"color","rgb":[256,0,0]}"#).is_err());
}

#[test]
fn numeric_settings_reject_nan_infinity_and_negative_values() {
    for threshold in [-1.0, 256.0, f64::NAN, f64::INFINITY] {
        let request = QuantizeRequest {
            alpha: AlphaPolicy::Preserve { threshold },
            ..quantize()
        };
        assert_eq!(
            Request::Quantize(request).validate().unwrap_err().path,
            "alpha.threshold"
        );
    }
    for strength in [-1.0, f32::NAN, f32::INFINITY] {
        let request = PerturbRequest {
            version: 1,
            source: source(),
            perturb: PerturbPolicy {
                strength,
                ..perturb()
            },
        };
        assert_eq!(
            Request::Perturb(request).validate().unwrap_err().path,
            "perturb.strength"
        );
    }
    let policy = PerturbPolicy {
        placement: Placement::Adaptive {
            radius: 0,
            threshold: 12.0,
            softness: 8.0,
        },
        ..perturb()
    };
    assert_eq!(
        Request::Perturb(PerturbRequest {
            version: 1,
            source: source(),
            perturb: policy
        })
        .validate()
        .unwrap_err()
        .path,
        "perturb.placement.radius"
    );
}

#[test]
fn indexed_storage_preserves_palette_order_transparency_and_warning_metadata() {
    let result = IndexedImage {
        indices: ImageBuf::<PaletteIndex8>::from_vec_packed(
            vec![1],
            ImageDimensions::new(1, 1).unwrap(),
        )
        .unwrap(),
        palette: NormalizedPalette {
            rgba: vec![30, 20, 10, 255, 0, 0, 0, 0],
            transparent_index: Some(1),
        },
        warnings: vec![ProcessWarning {
            code: WarningCode::TransparentOnly,
            message: "fixture".into(),
        }],
    };
    let mut copy = result.clone();
    copy.indices.data_mut()[0] = 0;
    copy.palette.rgba[0] = 255;
    assert_eq!(result.indices.data(), [1]);
    assert_eq!(result.palette.rgba, [30, 20, 10, 255, 0, 0, 0, 0]);
    assert_eq!(result.palette.transparent_index, Some(1));
    assert_eq!(result.warnings[0].code, WarningCode::TransparentOnly);
}
