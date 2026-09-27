//! Recipe-v2 production coverage resize equals the independent reference byte for byte.

use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{failure::Failure, request::RecipeV1},
        effects,
        pipeline::{
            process::ProcessRequest,
            processor::Processor,
            quantize::{IndexedMetadataRef, InputBoundary, QuantizeBoundary},
        },
    },
    spec::{self, contract::request::Source},
};
use serde_json::{json, Value};

const PALETTE: [PaletteEntry; 5] = [
    PaletteEntry::Transparent {},
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Color { rgb: [200, 40, 90] },
    PaletteEntry::Color {
        rgb: [40, 180, 120],
    },
    PaletteEntry::Color {
        rgb: [255, 255, 255],
    },
];

struct Io<'a>(&'a [u8]);

impl InputBoundary for Io<'_> {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        if compare && destination == self.0 {
            return Ok(true);
        }
        destination.copy_from_slice(self.0);
        Ok(false)
    }
}

impl QuantizeBoundary for Io<'_> {
    type Output = IndexedImage;
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        metadata: IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, Failure> {
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: metadata.palette.clone(),
            warnings: metadata.warnings.to_vec(),
        })
    }
}

fn source(opaque: bool) -> Vec<u8> {
    (0..9 * 7)
        .flat_map(|index| {
            let x = index % 9;
            let y = index / 9;
            let alpha = if opaque {
                255
            } else if (2..7).contains(&x) && (1..6).contains(&y) {
                ((x * 31 + y * 19) % 220 + 35) as u8
            } else {
                0
            };
            [
                (index * 17) as u8,
                (255 - index * 3) as u8,
                (index * 29) as u8,
                alpha,
            ]
        })
        .collect()
}

fn policies(anchor: &str) -> Vec<Value> {
    let mut values = vec![
        json!({ "algorithm": "nearest", "anchor": anchor }),
        json!({ "algorithm": "area" }),
        json!({ "algorithm": "bilinear", "anchor": anchor }),
        json!({ "algorithm": "trilinear", "anchor": anchor }),
    ];
    for support in ["fixed", "scale-aware"] {
        for algorithm in ["bicubic", "lanczos2", "lanczos3"] {
            values.push(json!({ "algorithm": algorithm, "anchor": anchor, "support": support }));
        }
    }
    values
}

fn dithers() -> [Value; 4] {
    [
        json!({ "family": "none" }),
        json!({ "family": "separable", "perturb": { "field": { "algorithm": "bayer", "size": "2" }, "space": "srgb", "strength": 0.5, "placement": { "mode": "everywhere" } } }),
        json!({ "family": "diffusion", "kernel": "floyd-steinberg", "strength": 0.8, "placement": { "mode": "everywhere" }, "serpentine": true, "feedback": "srgb-bytes" }),
        json!({ "family": "yliluoma", "size": "2", "placement": { "mode": "everywhere" } }),
    ]
}

fn terminal(recipe: &spec::effects::RecipeV2) -> RecipeV1 {
    let mut value = serde_json::to_value(recipe.terminal()).unwrap();
    value["version"] = json!(1);
    serde_json::from_value(value).unwrap()
}

#[test]
fn production_v2_matches_coverage_reference_matrix() {
    for opaque in [false, true] {
        let data = source(opaque);
        for anchor in ["top-left", "center", "bottom-right"] {
            for resize in policies(anchor) {
                for (width, height) in [(4, 3), (9, 7), (15, 12)] {
                    for effects_json in [
                        json!([]),
                        json!([{ "effect": "brightness-contrast", "enabled": true, "brightness": 0.05, "contrast": 0.1 }]),
                    ] {
                        for (dither, threshold) in dithers()
                            .into_iter()
                            .flat_map(|dither| [(dither.clone(), 0.0), (dither, 127.5)])
                        {
                            let recipe_json = json!({
                                "version": 2,
                                "effects": effects_json,
                                "output": { "width": width, "height": height, "resize": resize },
                                "alpha": { "mode": "preserve", "threshold": threshold },
                                "match": "oklab-euclidean",
                                "dither": dither,
                            });
                            let recipe =
                                spec::effects::decode_recipe_v2(&recipe_json.to_string()).unwrap();
                            let expected =
                                spec::effects::process(spec::effects::ProcessRequestV2 {
                                    source: Source {
                                        width: 9,
                                        height: 7,
                                        data: &data,
                                    },
                                    palette: &PALETTE,
                                    recipe: &recipe,
                                })
                                .unwrap();
                            let prod_effects =
                                effects::decode_effects(&effects_json.to_string()).unwrap();
                            let actual = Processor::new(64 * 1024 * 1024, 0)
                                .unwrap()
                                .process_effects(
                                    ProcessRequest {
                                        source_width: 9,
                                        source_height: 7,
                                        palette: &PALETTE,
                                        recipe: terminal(&recipe),
                                    },
                                    &prod_effects,
                                    &mut Io(&data),
                                )
                                .unwrap();
                            assert_eq!(actual, expected, "opaque={opaque} resize={resize} output={width}x{height} effects={effects_json} dither={dither} threshold={threshold}");
                        }
                    }
                }
            }
        }
    }
}
