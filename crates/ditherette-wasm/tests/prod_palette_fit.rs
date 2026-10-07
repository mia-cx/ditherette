//! Palette fit: production equals the frozen reference byte for byte.

use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::{
        contract::request::Source as ProdSource,
        effects::{self as prod, palette_fit},
    },
    spec::{
        contract::request::Source,
        effects::{self as spec, palette_fit as spec_fit},
    },
};
use serde_json::{json, Value};

const PALETTE: [PaletteEntry; 5] = [
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Transparent {},
    PaletteEntry::Color {
        rgb: [240, 200, 100],
    },
    PaletteEntry::Color { rgb: [90, 30, 140] },
    PaletteEntry::Color {
        rgb: [255, 255, 255],
    },
];

const FIT_SPACES: [(spec_fit::FitSpace, palette_fit::FitSpace, &str); 2] = [
    (
        spec_fit::FitSpace::Oklab,
        palette_fit::FitSpace::Oklab,
        "oklab",
    ),
    (
        spec_fit::FitSpace::Cielab,
        palette_fit::FitSpace::Cielab,
        "cielab",
    ),
];
const FIT_LOOKS: [(spec_fit::FitLook, palette_fit::FitLook, &str); 3] = [
    (
        spec_fit::FitLook::Natural,
        palette_fit::FitLook::Natural,
        "natural",
    ),
    (
        spec_fit::FitLook::Fitted,
        palette_fit::FitLook::Fitted,
        "fitted",
    ),
    (
        spec_fit::FitLook::Vivid,
        palette_fit::FitLook::Vivid,
        "vivid",
    ),
];

/// Saturated swatches, greys, near-greys, translucent pixels, and out-of-range leftovers.
fn fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rng = 0x9e3779b9u32;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng
    };
    (0..width * height)
        .flat_map(|pixel| {
            if pixel % 11 == 10 {
                [200, 40, 60, 128]
            } else if pixel % 7 == 3 {
                let grey = (next() % 256) as u8;
                [grey, grey, grey, 255]
            } else {
                [
                    (next() % 256) as u8,
                    (next() % 256) as u8,
                    (next() % 256) as u8,
                    255,
                ]
            }
        })
        .collect()
}

fn fit_look(look: &str, space: &str, strength: f32, curves: Value) -> Value {
    json!({ "effect": "palette-fit", "enabled": true, "look": look,
        "space": space, "strength": strength, "curves": curves })
}

fn fit(space: &str, strength: f32, curves: Value) -> Value {
    fit_look("fitted", space, strength, curves)
}

fn spec_run(data: &[u8], width: u32, height: u32, chain: &Value) -> Vec<u8> {
    spec::apply_effects(spec::EffectsRequest {
        version: 1,
        source: Source {
            width,
            height,
            data,
        },
        effects: &spec::decode_effects(&chain.to_string()).unwrap(),
        context: spec::EffectContext {
            palette: &PALETTE,
            space: None,
        },
    })
    .unwrap()
    .into_vec()
}

fn prod_run(data: &[u8], width: u32, height: u32, chain: &Value) -> Vec<u8> {
    prod::apply_effects(prod::EffectsRequest {
        version: 1,
        source: ProdSource {
            width,
            height,
            data,
        },
        effects: &prod::decode_effects(&chain.to_string()).unwrap(),
        context: prod::EffectContext {
            palette: &PALETTE,
            space: None,
            analyses: None,
        },
    })
    .unwrap()
    .into_vec()
}

#[test]
fn fitted_chains_match_the_reference() {
    for (_spec_space, _prod_space, tag) in FIT_SPACES {
        for (_spec_look, _prod_look, look) in FIT_LOOKS {
            for (width, height, data) in [(16, 16, fixture(16, 16)), (9, 7, fixture(9, 7))] {
                for chain in [
                    json!([fit_look(look, tag, 1.0, Value::Null)]),
                    json!([fit_look(look, tag, 0.55, Value::Null)]),
                    // Before, after, masked, disabled, and doubled steps.
                    json!([
                        { "effect": "exposure", "enabled": true, "stops": -0.4 },
                        fit_look(look, tag, 0.9, Value::Null),
                        { "effect": "brightness-contrast", "enabled": true,
                          "brightness": 0.1, "contrast": 0.2 }
                    ]),
                    json!([
                        { "effect": "palette-fit", "enabled": true, "look": look, "space": tag,
                          "strength": 1.0, "curves": null,
                          "mask": [{ "x": { "model": "oklch", "channel": "lightness" },
                                     "points": [[0, 1], [1, 0.3]] }] }
                    ]),
                    json!([
                        { "effect": "palette-fit", "enabled": false, "look": look, "space": tag,
                          "strength": 1.0, "curves": null },
                        fit_look(look, tag, 1.0, Value::Null),
                    ]),
                ] {
                    assert_eq!(
                        spec_run(&data, width, height, &chain),
                        prod_run(&data, width, height, &chain),
                        "{look} {tag} {width}x{height} {chain}"
                    );
                }
            }
        }
    }
}

#[test]
fn explicit_lists_match_the_reference() {
    // Hand-edited-looking curves, including models outside the step's pair.
    let curves = json!([
        { "kind": "remap", "x": { "model": "oklab", "channel": "lightness" },
          "y": { "model": "oklab", "channel": "lightness" },
          "points": [[0, 0.05], [0.5, 0.45], [1, 0.9]] },
        { "kind": "adjust", "x": { "model": "hsl", "channel": "lightness" },
          "y": { "model": "hsv", "channel": "saturation" },
          "points": [[0, 0.7], [1, 0.5]] }
    ]);
    let data = fixture(12, 10);
    for (_, _, tag) in FIT_SPACES {
        for strength in [0.0, 0.4, 1.0] {
            let chain = json!([fit(tag, strength, curves.clone())]);
            assert_eq!(
                spec_run(&data, 12, 10, &chain),
                prod_run(&data, 12, 10, &chain),
                "{tag} strength {strength}"
            );
        }
    }
}

#[test]
fn analysis_matches_the_reference_in_both_spaces() {
    for (width, height, data) in [(16, 16, fixture(16, 16)), (9, 7, fixture(9, 7))] {
        for (spec_space, prod_space, _tag) in FIT_SPACES {
            for (spec_look, prod_look, look_tag) in FIT_LOOKS {
                let expected = spec::analyze_palette_fit(spec::AnalyzePaletteFitRequest {
                    version: 1,
                    source: Source {
                        width,
                        height,
                        data: &data,
                    },
                    effects: &[],
                    context: spec::EffectContext {
                        palette: &PALETTE,
                        space: None,
                    },
                    space: spec_space,
                    look: spec_look,
                })
                .unwrap();
                let actual = prod::analyze_palette_fit(prod::AnalyzePaletteFitRequest {
                    version: 1,
                    source: ProdSource {
                        width,
                        height,
                        data: &data,
                    },
                    effects: &[],
                    context: prod::EffectContext {
                        palette: &PALETTE,
                        space: None,
                        analyses: None,
                    },
                    space: prod_space,
                    look: prod_look,
                })
                .unwrap();
                assert_eq!(
                    serde_json::to_string(&actual).unwrap(),
                    serde_json::to_string(&expected).unwrap(),
                    "{spec_space:?} {look_tag}"
                );
            }
        }
    }
}

#[test]
fn stepwise_and_folded_application_agree() {
    let data = fixture(11, 9);
    let chain_json = json!([
        { "effect": "levels", "enabled": true, "channel": "rgb",
          "input": { "black": 0.05, "white": 0.95 }, "gamma": 1.1,
          "output": { "black": 0, "white": 1 } },
        fit("oklab", 0.8, Value::Null),
        fit("cielab", 0.6, json!([
            { "kind": "remap", "x": { "model": "oklch", "channel": "chroma" },
              "y": { "model": "oklch", "channel": "chroma" },
              "points": [[0, 0], [1, 0.6]] }
        ])),
    ]);
    let chain = prod::decode_effects(&chain_json.to_string()).unwrap();
    let dimensions = ditherette_wasm::image::ImageDimensions::new(11, 9).unwrap();
    let context = prod::EffectContext {
        palette: &PALETTE,
        space: None,
        analyses: None,
    };
    let mut in_place = data.clone();
    prod::apply_in_place(&mut in_place, dimensions, &chain, &context).unwrap();
    let view = ditherette_wasm::image::ImageView::packed(&data, dimensions).unwrap();
    let mut stepwise = prod::EffectImage::from_rgba8(view);
    prod::apply_chain(&mut stepwise, &chain, &context).unwrap();
    assert_eq!(in_place, stepwise.to_rgba8().data());
    assert_eq!(in_place, spec_run(&data, 11, 9, &chain_json));
}
