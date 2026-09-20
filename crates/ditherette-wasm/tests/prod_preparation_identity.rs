use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::{
        contract::request::{AlphaPolicy, MatchPolicy, Output, RECIPE_VERSION},
        pipeline::identity::{palette, resize},
    },
    spec::contract::{cache as frozen, request},
};

#[test]
fn streaming_keys_match_frozen_normalized_settings() {
    let palettes = [
        vec![
            PaletteEntry::Color { rgb: [1, 2, 3] },
            PaletteEntry::Transparent {},
        ],
        vec![
            PaletteEntry::Transparent {},
            PaletteEntry::Color { rgb: [1, 2, 3] },
        ],
        vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 256],
        vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 257],
    ];
    for entries in &palettes {
        for matching in [
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
        ] {
            for alpha in [
                AlphaPolicy::Preserve { threshold: -0.0 },
                AlphaPolicy::Preserve { threshold: 0.0 },
                AlphaPolicy::Preserve {
                    threshold: 127.9999999,
                },
                AlphaPolicy::Preserve { threshold: 128.0 },
                AlphaPolicy::Premultiplied {},
                AlphaPolicy::Matte { rgb: [3, 19, 47] },
            ] {
                let matching: MatchPolicy =
                    serde_json::from_value(serde_json::json!(matching)).unwrap();
                let expected = frozen::stage_identity(
                    None,
                    RECIPE_VERSION,
                    frozen::StageOptions::PreparedPalette {
                        palette: frozen::palette_identity(entries),
                        alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap())
                            .unwrap(),
                        matching: serde_json::from_value(serde_json::to_value(matching).unwrap())
                            .unwrap(),
                    },
                );
                assert_eq!(palette(entries, alpha, matching).unwrap().0, expected.0);
            }
        }
    }
    for algorithm in [
        "nearest",
        "area",
        "bilinear",
        "bicubic",
        "lanczos2",
        "lanczos3",
        "trilinear",
    ] {
        for anchor in [
            "top-left",
            "top",
            "top-right",
            "left",
            "center",
            "right",
            "bottom-left",
            "bottom",
            "bottom-right",
        ] {
            for support in ["fixed", "scale-aware"] {
                let mut settings = serde_json::json!({"algorithm": algorithm});
                if algorithm != "area" {
                    settings["anchor"] = serde_json::json!(anchor);
                }
                if ["bicubic", "lanczos2", "lanczos3"].contains(&algorithm) {
                    settings["support"] = serde_json::json!(support);
                }
                let output = Output {
                    width: 17,
                    height: 5,
                    resize: serde_json::from_value(settings).unwrap(),
                };
                let expected = frozen::stage_identity(
                    None,
                    RECIPE_VERSION,
                    frozen::StageOptions::ResizePlan {
                        source_width: 31,
                        source_height: 9,
                        output: serde_json::from_value::<request::Output>(
                            serde_json::to_value(output).unwrap(),
                        )
                        .unwrap(),
                    },
                );
                assert_eq!(resize(31, 9, output).unwrap().0, expected.0);
            }
        }
    }
}
