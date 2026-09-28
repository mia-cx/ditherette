//! Production effects must equal the frozen reference byte-for-byte.

use ditherette_wasm::prod::effects::Effect as _;
use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::{contract::request::Source as ProdSource, effects as prod},
    spec::{contract::request::Source, effects as spec},
};
use serde_json::{json, Value};

const PALETTE: [PaletteEntry; 3] = [
    PaletteEntry::Color { rgb: [10, 20, 30] },
    PaletteEntry::Transparent {},
    PaletteEntry::Color {
        rgb: [240, 200, 100],
    },
];

/// Every working space, so recolour steps analyse and apply in each across fixtures.
const SPACES: [ditherette_wasm::spec::contract::request::WorkingSpace; 7] = {
    use ditherette_wasm::spec::contract::request::WorkingSpace::*;
    [Srgb, LinearRgb, Oklab, Oklch, Cielab, Cielch, Ycbcr]
};
const PROD_SPACES: [ditherette_wasm::prod::contract::request::WorkingSpace; 7] = {
    use ditherette_wasm::prod::contract::request::WorkingSpace::*;
    [Srgb, LinearRgb, Oklab, Oklch, Cielab, Cielch, Ycbcr]
};
const MODEL_CURVE_MODELS: [&str; 8] = [
    "linear-rgb",
    "hsl",
    "hsv",
    "oklab",
    "oklch",
    "cielab",
    "cielch",
    "ycbcr",
];
const COLOUR_CHANNELS: [(&str, &str); 27] = [
    ("srgb", "red"),
    ("srgb", "green"),
    ("srgb", "blue"),
    ("linear-rgb", "red"),
    ("linear-rgb", "green"),
    ("linear-rgb", "blue"),
    ("hsl", "hue"),
    ("hsl", "saturation"),
    ("hsl", "lightness"),
    ("hsv", "hue"),
    ("hsv", "saturation"),
    ("hsv", "value"),
    ("oklab", "lightness"),
    ("oklab", "a"),
    ("oklab", "b"),
    ("oklch", "lightness"),
    ("oklch", "chroma"),
    ("oklch", "hue"),
    ("cielab", "lightness"),
    ("cielab", "a"),
    ("cielab", "b"),
    ("cielch", "lightness"),
    ("cielch", "chroma"),
    ("cielch", "hue"),
    ("ycbcr", "luma"),
    ("ycbcr", "cb"),
    ("ycbcr", "cr"),
];

/// Deterministic xorshift so failures reproduce.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next() % 10_001) as f32 / 10_000.0
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

fn image(rng: &mut Rng, width: u32, height: u32) -> Vec<u8> {
    (0..width * height * 4).map(|_| rng.next() as u8).collect()
}

/// Every byte value on every channel, then random pixels.
fn fixtures(rng: &mut Rng) -> Vec<(u32, u32, Vec<u8>)> {
    let ramp = (0..=255u8)
        .flat_map(|value| [value, 255 - value, value.wrapping_mul(7), value])
        .collect();
    vec![
        (16, 16, ramp),
        (1, 1, image(rng, 1, 1)),
        (7, 3, image(rng, 7, 3)),
        (33, 17, image(rng, 33, 17)),
    ]
}

/// Levels only: the per-channel effect the carrier-path test mixes with `Swap`.
fn random_levels(rng: &mut Rng) -> Value {
    let black = rng.unit() * 0.6;
    let white = black + 0.05 + rng.unit() * (0.95 - black);
    let gamma = if rng.next() % 2 == 0 {
        1.0
    } else {
        0.1 + rng.unit() * 9.9
    };
    json!({
        "effect": "levels",
        "enabled": rng.next() % 5 != 0,
        "channel": rng.pick(&["rgb", "red", "green", "blue"]),
        "input": { "black": black, "white": white.min(1.0) },
        "gamma": gamma,
        "output": { "black": rng.unit(), "white": rng.unit() },
    })
}

/// Any built-in with random in-range arguments; about one in four is neutral.
fn random_effect(rng: &mut Rng) -> Value {
    let enabled = rng.next() % 5 != 0;
    let neutral = rng.next() % 4 == 0;
    let signed = |rng: &mut Rng| if neutral { 0.0 } else { rng.unit() * 2.0 - 1.0 };
    match rng.next() % 9 {
        0 => random_levels(rng),
        1 => {
            let count = 2 + rng.next() % 5;
            let points: Vec<[f32; 2]> = (0..count)
                .map(|i| [i as f32 / (count - 1) as f32, rng.unit()])
                .collect();
            json!({ "effect": "curves", "enabled": enabled,
                "channel": rng.pick(&["rgb", "red", "green", "blue"]), "points": points })
        }
        2 => json!({ "effect": "brightness-contrast", "enabled": enabled,
            "brightness": signed(rng), "contrast": signed(rng) }),
        3 => json!({ "effect": "exposure", "enabled": enabled, "stops": signed(rng) * 4.0 }),
        4 => json!({ "effect": "white-balance", "enabled": enabled,
            "temperature": signed(rng), "tint": signed(rng) }),
        5 => json!({ "effect": "hue-saturation", "enabled": enabled,
            "hue": signed(rng) * 180.0, "saturation": signed(rng), "lightness": signed(rng) }),
        6 => json!({ "effect": "recolour", "enabled": enabled,
            "strength": if neutral { 0.0 } else { rng.unit() }, "recipe": null }),
        7 => {
            let identity = [[0.0, 0.0], [1.0, 1.0]];
            let bent = [[0.0, 0.0], [0.5, rng.unit()], [1.0, 1.0]];
            json!({ "effect": "model-curves", "enabled": enabled,
                "model": rng.pick(&MODEL_CURVE_MODELS),
                "curves": if neutral { [identity.as_slice(), identity.as_slice(), identity.as_slice()] }
                else { [bent.as_slice(), identity.as_slice(), identity.as_slice()] } })
        }
        _ => {
            let x = COLOUR_CHANNELS[(rng.next() % COLOUR_CHANNELS.len() as u64) as usize];
            let y = COLOUR_CHANNELS[(rng.next() % COLOUR_CHANNELS.len() as u64) as usize];
            let seam = if neutral { 0.5 } else { rng.unit() };
            let points = if x.1 == "hue" {
                vec![
                    [0.0, seam],
                    [0.5, if neutral { 0.5 } else { rng.unit() }],
                    [1.0, seam],
                ]
            } else if neutral {
                vec![[0.0, 0.5], [1.0, 0.5]]
            } else {
                vec![[0.0, rng.unit()], [0.5, rng.unit()], [1.0, rng.unit()]]
            };
            json!({ "effect": "channel-curve", "enabled": enabled,
                "x": { "model": x.0, "channel": x.1 },
                "y": { "model": y.0, "channel": y.1 }, "points": points })
        }
    }
}

fn assert_same(effects: &Value, width: u32, height: u32, data: &[u8]) {
    let json = effects.to_string();
    let spec_steps = spec::decode_effects(&json).unwrap();
    let prod_steps = prod::decode_effects(&json).unwrap();
    let expected = spec::apply_effects(spec::EffectsRequest {
        version: 1,
        source: Source {
            width,
            height,
            data,
        },
        effects: &spec_steps,
        context: spec::EffectContext {
            palette: &PALETTE,
            space: Some(SPACES[(data.len() / 4) % SPACES.len()]),
        },
    });
    let actual = prod::apply_effects(prod::EffectsRequest {
        version: 1,
        source: ProdSource {
            width,
            height,
            data,
        },
        effects: &prod_steps,
        context: prod::EffectContext {
            palette: &PALETTE,
            space: Some(PROD_SPACES[(data.len() / 4) % PROD_SPACES.len()]),
            analyses: None,
        },
    });
    match (expected, actual) {
        (Ok(expected), Ok(actual)) => {
            assert_eq!(actual.data(), expected.data(), "{json}");
            assert_eq!(actual.dimensions(), expected.dimensions());
        }
        (Err(expected), Err(actual)) => {
            assert_eq!(
                (format!("{:?}", actual.code), actual.path, actual.message),
                (
                    format!("{:?}", expected.code),
                    expected.path,
                    expected.message
                ),
                "{json}"
            );
        }
        (expected, actual) => panic!("{json}: spec {expected:?} but prod {actual:?}"),
    }
}

#[test]
fn random_chains_match_the_reference() {
    let mut rng = Rng(0x5eed_ef1e_c7);
    for (width, height, data) in fixtures(&mut rng) {
        assert_same(&json!([]), width, height, &data);
        for _ in 0..150 {
            let count = rng.next() % 6;
            let effects: Vec<Value> = (0..count).map(|_| random_effect(&mut rng)).collect();
            assert_same(&Value::Array(effects), width, height, &data);
        }
    }
}

#[test]
fn randomized_model_curves_match_the_reference_for_every_model() {
    let mut rng = Rng(0xc01a_267);
    let data = image(&mut rng, 37, 19);
    for model in MODEL_CURVE_MODELS {
        for _ in 0..24 {
            let curves: Vec<Vec<[f32; 2]>> = (0..3)
                .map(|_| {
                    vec![
                        [0.0, rng.unit()],
                        [0.3, rng.unit()],
                        [0.7, rng.unit()],
                        [1.0, rng.unit()],
                    ]
                })
                .collect();
            let effects = json!([
                random_levels(&mut rng),
                { "effect": "model-curves", "enabled": true, "model": model, "curves": curves },
                { "effect": "brightness-contrast", "enabled": true,
                  "brightness": rng.unit() * 0.4 - 0.2, "contrast": rng.unit() * 0.4 - 0.2 }
            ]);
            assert_same(&effects, 37, 19, &data);
        }
    }
}

#[test]
fn randomized_cross_model_channel_curves_match_the_reference() {
    let mut rng = Rng(0xc01a_268);
    let data = image(&mut rng, 37, 19);
    for x in COLOUR_CHANNELS {
        for _ in 0..12 {
            let y = COLOUR_CHANNELS[(rng.next() % COLOUR_CHANNELS.len() as u64) as usize];
            let seam = rng.unit();
            let points = if x.1 == "hue" {
                vec![
                    [0.0, seam],
                    [0.3, rng.unit()],
                    [0.7, rng.unit()],
                    [1.0, seam],
                ]
            } else {
                vec![
                    [0.0, rng.unit()],
                    [0.3, rng.unit()],
                    [0.7, rng.unit()],
                    [1.0, rng.unit()],
                ]
            };
            let effects = json!([
                random_levels(&mut rng),
                { "effect": "channel-curve", "enabled": true,
                  "x": { "model": x.0, "channel": x.1 },
                  "y": { "model": y.0, "channel": y.1 }, "points": points },
                { "effect": "exposure", "enabled": true, "stops": rng.unit() - 0.5 }
            ]);
            assert_same(&effects, 37, 19, &data);
        }
    }
}

#[test]
fn large_repeated_colour_effects_match_the_reference() {
    let effects = json!([
        { "effect": "hue-saturation", "enabled": true,
          "hue": 25, "saturation": 0.3, "lightness": 0.05 },
        { "effect": "model-curves", "enabled": true, "model": "oklch",
          "curves": [[[0, 0], [1, 1]], [[0, 0], [0.5, 0.65], [1, 1]], [[0, 0], [1, 1]]] },
        { "effect": "channel-curve", "enabled": true,
          "x": { "model": "hsv", "channel": "value" },
          "y": { "model": "cielch", "channel": "chroma" },
          "points": [[0, 0.2], [0.5, 0.8], [1, 0.5]] }
    ]);
    let data: Vec<u8> = (0..512 * 384)
        .flat_map(|index| {
            let rgb = [[17, 31, 47], [190, 80, 23], [240, 240, 240]][index % 3];
            [rgb[0], rgb[1], rgb[2], index as u8]
        })
        .collect();
    assert_same(&effects, 512, 384, &data);
}

#[test]
fn high_cardinality_effects_match_the_reference() {
    let effects = json!([
        { "effect": "curves", "enabled": true, "channel": "rgb",
          "points": [[0, 0], [0.25, 0.2], [0.75, 0.85], [1, 1]] },
        { "effect": "hue-saturation", "enabled": true,
          "hue": -137, "saturation": 0.65, "lightness": -0.2 },
        { "effect": "model-curves", "enabled": true, "model": "cielch",
          "curves": [[[0, 0], [1, 1]], [[0, 0], [0.4, 0.7], [1, 1]], [[0, 0.1], [1, 0.9]]] },
        { "effect": "channel-curve", "enabled": true,
          "x": { "model": "oklch", "channel": "hue" },
          "y": { "model": "hsl", "channel": "lightness" },
          "points": [[0, 0.3], [0.25, 0.9], [0.75, 0.1], [1, 0.3]] }
    ]);
    let data: Vec<u8> = (0..512 * 512u32)
        .flat_map(|index| {
            let bytes = index.to_le_bytes();
            [bytes[0], bytes[1], bytes[2], (index * 37) as u8]
        })
        .collect();
    assert_same(&effects, 512, 512, &data);
}

#[test]
fn linear_rgb_table_matches_direct_reference_for_every_channel_byte() {
    use ditherette_wasm::prod::effects::model_curves::{ModelCurves, ModelCurvesModel};

    let curves = [
        vec![[0.0, 0.1], [0.4, 0.7], [1.0, 0.9]],
        vec![[0.0, 0.0], [0.6, 0.3], [1.0, 1.0]],
        vec![[0.0, 0.2], [1.0, 0.8]],
    ];
    let production = ModelCurves {
        model: ModelCurvesModel::LinearRgb,
        curves: curves.clone(),
    };
    let reference = spec::model_curves::ModelCurves {
        model: spec::model_curves::ModelCurvesModel::LinearRgb,
        curves,
    };
    let dimensions = ditherette_wasm::image::ImageDimensions::new(1, 1).unwrap();
    for channel in 0..3 {
        for value in 0..=u8::MAX {
            let unit = value as f32 / 255.0;
            let mut image = spec::EffectImage {
                dimensions,
                rgb: vec![[unit; 3]],
                alpha: vec![255],
            };
            spec::Effect::apply(&reference, &mut image, &spec::EffectContext::default());
            assert_eq!(
                production.map_channel(channel, unit).to_bits(),
                image.rgb[0][channel].to_bits(),
                "channel {channel}, byte {value}"
            );
        }
    }
}

#[test]
fn eligible_channel_curve_tables_match_direct_reference_for_every_byte() {
    let models = ["srgb", "linear-rgb"];
    let channels = ["red", "green", "blue"];
    for x_model in models {
        for y_model in models {
            for (channel, name) in channels.iter().enumerate() {
                let json = json!([{
                    "effect": "channel-curve", "enabled": true,
                    "x": { "model": x_model, "channel": name },
                    "y": { "model": y_model, "channel": name },
                    "points": [[0, 0.1], [0.4, 0.8], [1, 0.6]]
                }]);
                let production = prod::decode_effects(&json.to_string())
                    .unwrap()
                    .remove(0)
                    .effect;
                let reference = spec::decode_effects(&json.to_string())
                    .unwrap()
                    .remove(0)
                    .effect;
                assert!(production.per_channel());
                let dimensions = ditherette_wasm::image::ImageDimensions::new(1, 1).unwrap();
                for value in 0..=u8::MAX {
                    let unit = value as f32 / 255.0;
                    let mut image = spec::EffectImage {
                        dimensions,
                        rgb: vec![[unit; 3]],
                        alpha: vec![255],
                    };
                    spec::Effect::apply(&reference, &mut image, &spec::EffectContext::default());
                    assert_eq!(
                        production.map_channel(channel, unit).to_bits(),
                        image.rgb[0][channel].to_bits(),
                        "{x_model} to {y_model} {name}, byte {value}"
                    );
                }
            }
        }
    }
}

#[test]
fn prepared_hue_matches_frozen_map_for_byte_inputs() {
    use ditherette_wasm::prod::effects::{
        chain::PreparedPointwiseState, hue_saturation::HueSaturation, table::ChannelTables,
    };

    let context = prod::EffectContext::default();
    for effect in hue_effects() {
        let reference = reference_hue(effect);
        let tables = ChannelTables::new(std::iter::empty::<&HueSaturation>());
        let effects = [&effect];
        let prepared = PreparedPointwiseState::new(&effects, &tables);
        for red in (0..=u8::MAX).step_by(17) {
            for green in (0..=u8::MAX).step_by(29) {
                for blue in (0..=u8::MAX).step_by(43) {
                    let bytes = [red, green, blue];
                    let input = bytes.map(|channel| channel as f32 / 255.0);
                    assert_float_bits(prepared.map(bytes, &context), reference.map(input));
                }
            }
        }
    }
}

#[test]
fn prepared_hue_matches_frozen_map_for_carrier_values() {
    let context = prod::EffectContext::default();
    let inputs = [
        [-64.0, -0.5, 1.25],
        [-0.125, 0.0, 1.0],
        [0.003_130_8, 0.04045, 1.5],
        [2.0, 8.0, 64.0],
    ];
    for effect in hue_effects() {
        let reference = reference_hue(effect);
        let prepared = effect.prepare_pointwise();
        for input in inputs {
            assert_float_bits(
                effect.map_prepared(prepared, input, &context),
                reference.map(input),
            );
        }
    }
}

#[test]
fn prepared_neutral_hue_is_an_exact_identity() {
    let effect = prod::hue_saturation::HueSaturation {
        hue: 0.0,
        saturation: 0.0,
        lightness: 0.0,
    };
    let prepared = effect.prepare_pointwise();
    let context = prod::EffectContext::default();
    for input in [[0.0, 0.5, 1.0], [-64.0, -0.25, 64.0]] {
        assert_float_bits(effect.map_prepared(prepared, input, &context), input);
    }
}

fn hue_effects() -> [prod::hue_saturation::HueSaturation; 8] {
    use prod::hue_saturation::HueSaturation;

    [
        HueSaturation {
            hue: -180.0,
            saturation: -1.0,
            lightness: -1.0,
        },
        HueSaturation {
            hue: 180.0,
            saturation: 1.0,
            lightness: 1.0,
        },
        HueSaturation {
            hue: -25.0,
            saturation: 0.3,
            lightness: -0.55,
        },
        HueSaturation {
            hue: 25.0,
            saturation: 0.3,
            lightness: 0.55,
        },
        HueSaturation {
            hue: 0.0,
            saturation: -1.0,
            lightness: 0.0,
        },
        HueSaturation {
            hue: 0.0,
            saturation: 1.0,
            lightness: 0.0,
        },
        HueSaturation {
            hue: 0.0,
            saturation: 0.0,
            lightness: -1.0,
        },
        HueSaturation {
            hue: 0.0,
            saturation: 0.0,
            lightness: 1.0,
        },
    ]
}

fn reference_hue(
    effect: prod::hue_saturation::HueSaturation,
) -> spec::hue_saturation::HueSaturation {
    spec::hue_saturation::HueSaturation {
        hue: effect.hue,
        saturation: effect.saturation,
        lightness: effect.lightness,
    }
}

fn assert_float_bits(actual: [f32; 3], expected: [f32; 3]) {
    assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
}

#[test]
fn invalid_chains_fail_identically() {
    let mut rng = Rng(7);
    let (width, height, data) = fixtures(&mut rng).remove(2);
    for invalid in [
        json!([{ "effect": "levels", "enabled": false, "channel": "rgb",
            "input": { "black": 0.5, "white": 0.5 }, "gamma": 1.0,
            "output": { "black": 0.0, "white": 1.0 } }]),
        json!([{ "effect": "levels", "enabled": true, "channel": "rgb",
            "input": { "black": 0.0, "white": 1.0 }, "gamma": 11.0,
            "output": { "black": 0.0, "white": 1.0 } }]),
    ] {
        assert_same(&invalid, width, height, &data);
    }
}

/// Not per-channel: forces the continuous carrier after a tabulated prefix.
struct Swap;

impl prod::Effect for Swap {
    fn validate(
        &self,
        _path: &str,
    ) -> Result<(), ditherette_wasm::prod::contract::error::DitheretteError> {
        Ok(())
    }

    fn apply(&self, image: &mut prod::EffectImage, _context: &prod::EffectContext<'_>) {
        for rgb in &mut image.rgb {
            *rgb = [rgb[2] * 1.5 - 0.2, rgb[0], rgb[1]];
        }
    }
}

#[test]
fn tabulated_prefixes_feed_the_carrier_exactly() {
    let mut rng = Rng(99);
    for (width, height, data) in fixtures(&mut rng) {
        for _ in 0..20 {
            let mut chain: Vec<prod::Step<Box<dyn prod::Effect>>> = Vec::new();
            for _ in 0..rng.next() % 6 {
                let effect: Box<dyn prod::Effect> = if rng.next() % 3 == 0 {
                    Box::new(Swap)
                } else {
                    let step = prod::decode_effects(&json!([random_levels(&mut rng)]).to_string())
                        .unwrap()
                        .remove(0);
                    Box::new(step.effect)
                };
                chain.push(prod::Step {
                    enabled: rng.next() % 5 != 0,
                    effect,
                });
            }
            let dimensions = ditherette_wasm::image::ImageDimensions::new(width, height).unwrap();
            let view = ditherette_wasm::image::ImageView::packed(&data, dimensions).unwrap();
            let mut stepwise = prod::EffectImage::from_rgba8(view);
            let context = prod::EffectContext::default();
            prod::apply_chain(&mut stepwise, &chain, &context).unwrap();
            let mut folded = data.clone();
            prod::apply_in_place(&mut folded, dimensions, &chain, &context).unwrap();
            assert_eq!(folded, stepwise.to_rgba8().data());
        }
    }
}

#[test]
fn analysis_matches_the_reference_in_every_space() {
    let mut rng = Rng(2024);
    let palettes: [&[[u8; 3]]; 3] = [
        &[[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]],
        &[[96, 0, 24], [237, 28, 36], [255, 127, 39], [249, 221, 59]],
        &[
            [15, 56, 15],
            [48, 98, 48],
            [139, 172, 15],
            [155, 188, 15],
            [40, 80, 158],
        ],
    ];
    for (width, height, data) in fixtures(&mut rng).into_iter().skip(1) {
        for colors in palettes {
            let palette: Vec<PaletteEntry> = colors
                .iter()
                .map(|&rgb| PaletteEntry::Color { rgb })
                .collect();
            for (space, prod_space) in SPACES.into_iter().zip(PROD_SPACES) {
                let expected = spec::analyze_recolour(spec::AnalyzeRequest {
                    version: 1,
                    source: Source {
                        width,
                        height,
                        data: &data,
                    },
                    effects: &[],
                    context: spec::EffectContext {
                        palette: &palette,
                        space: Some(space),
                    },
                })
                .unwrap();
                let actual = prod::analyze_recolour(prod::AnalyzeRequest {
                    version: 1,
                    source: ProdSource {
                        width,
                        height,
                        data: &data,
                    },
                    effects: &[],
                    context: prod::EffectContext {
                        palette: &palette,
                        space: Some(prod_space),
                        analyses: None,
                    },
                })
                .unwrap();
                assert_eq!(
                    serde_json::to_string(&actual).unwrap(),
                    serde_json::to_string(&expected).unwrap(),
                    "{space:?}"
                );
            }
        }
    }
}
