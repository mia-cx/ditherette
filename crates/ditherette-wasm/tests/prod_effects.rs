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

fn model_channels(model: &str) -> [&'static str; 3] {
    match model {
        "linear-rgb" => ["red", "green", "blue"],
        "hsl" => ["hue", "saturation", "lightness"],
        "hsv" => ["hue", "saturation", "value"],
        "oklab" | "cielab" => ["lightness", "a", "b"],
        "oklch" | "cielch" => ["lightness", "chroma", "hue"],
        "ycbcr" => ["luma", "cb", "cr"],
        _ => unreachable!("unsupported model fixture"),
    }
}
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
                .map(|i| {
                    let x = i as f32 / (count - 1) as f32;
                    [x, if neutral { x } else { rng.unit() }]
                })
                .collect();
            let channel = rng.pick(&["red", "green", "blue"]);
            json!({ "effect": "curves", "enabled": enabled,
                "curves": [{ "kind": "remap",
                    "x": { "model": "srgb", "channel": channel },
                    "y": { "model": "srgb", "channel": channel }, "points": points }] })
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
            let closed_hue = [[0.0, 0.0], [1.0, 0.0]];
            let bent = [[0.0, 0.0], [0.5, rng.unit()], [1.0, 1.0]];
            let model = rng.pick(&MODEL_CURVE_MODELS);
            let curves: Vec<_> = model_channels(model)
                .into_iter()
                .enumerate()
                .map(|(index, channel)| {
                    let points = if channel == "hue" {
                        closed_hue.as_slice()
                    } else if neutral || index != 0 {
                        identity.as_slice()
                    } else {
                        bent.as_slice()
                    };
                    json!({ "kind": "remap",
                        "x": { "model": model, "channel": channel },
                        "y": { "model": model, "channel": channel }, "points": points })
                })
                .collect();
            json!({ "effect": "curves", "enabled": enabled,
                "curves": curves })
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
            json!({ "effect": "curves", "enabled": enabled, "curves": [{
                "kind": "adjust", "x": { "model": x.0, "channel": x.1 },
                "y": { "model": y.0, "channel": y.1 }, "points": points }] })
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
fn randomized_remaps_match_the_reference_for_every_model() {
    let mut rng = Rng(0xc01a_267);
    let data = image(&mut rng, 37, 19);
    for model in MODEL_CURVE_MODELS {
        for _ in 0..24 {
            let curves: Vec<_> = model_channels(model)
                .into_iter()
                .map(|channel| {
                    let seam = rng.unit();
                    let points = if channel == "hue" {
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
                    json!({ "kind": "remap",
                        "x": { "model": model, "channel": channel },
                        "y": { "model": model, "channel": channel }, "points": points })
                })
                .collect();
            let effects = json!([
                random_levels(&mut rng),
                { "effect": "curves", "enabled": true, "curves": curves },
                { "effect": "brightness-contrast", "enabled": true,
                  "brightness": rng.unit() * 0.4 - 0.2, "contrast": rng.unit() * 0.4 - 0.2 }
            ]);
            assert_same(&effects, 37, 19, &data);
        }
    }
}

#[test]
fn randomized_cross_model_adjustments_match_the_reference() {
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
                { "effect": "curves", "enabled": true, "curves": [{
                  "kind": "adjust", "x": { "model": x.0, "channel": x.1 },
                  "y": { "model": y.0, "channel": y.1 }, "points": points }] },
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
        { "effect": "curves", "enabled": true, "curves": [
          { "kind": "remap", "x": { "model": "oklch", "channel": "lightness" },
            "y": { "model": "oklch", "channel": "lightness" }, "points": [[0, 0], [1, 1]] },
          { "kind": "remap", "x": { "model": "oklch", "channel": "chroma" },
            "y": { "model": "oklch", "channel": "chroma" }, "points": [[0, 0], [0.5, 0.65], [1, 1]] },
          { "kind": "adjust", "x": { "model": "hsv", "channel": "value" },
            "y": { "model": "cielch", "channel": "chroma" },
            "points": [[0, 0.2], [0.5, 0.8], [1, 0.5]] }
        ] }
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
        { "effect": "curves", "enabled": true, "curves": [
          { "kind": "remap", "x": { "model": "srgb", "channel": "red" },
            "y": { "model": "srgb", "channel": "red" },
            "points": [[0, 0], [0.25, 0.2], [0.75, 0.85], [1, 1]] },
          { "kind": "remap", "x": { "model": "srgb", "channel": "green" },
            "y": { "model": "srgb", "channel": "green" },
            "points": [[0, 0], [0.25, 0.2], [0.75, 0.85], [1, 1]] },
          { "kind": "remap", "x": { "model": "srgb", "channel": "blue" },
            "y": { "model": "srgb", "channel": "blue" },
            "points": [[0, 0], [0.25, 0.2], [0.75, 0.85], [1, 1]] }
        ] },
        { "effect": "hue-saturation", "enabled": true,
          "hue": -137, "saturation": 0.65, "lightness": -0.2 },
        { "effect": "curves", "enabled": true, "curves": [
          { "kind": "remap", "x": { "model": "cielch", "channel": "lightness" },
            "y": { "model": "cielch", "channel": "lightness" }, "points": [[0, 0], [1, 1]] },
          { "kind": "remap", "x": { "model": "cielch", "channel": "chroma" },
            "y": { "model": "cielch", "channel": "chroma" }, "points": [[0, 0], [0.4, 0.7], [1, 1]] },
          { "kind": "remap", "x": { "model": "cielch", "channel": "hue" },
            "y": { "model": "cielch", "channel": "hue" }, "points": [[0, 0.1], [1, 0.1]] },
          { "kind": "adjust", "x": { "model": "oklch", "channel": "hue" },
            "y": { "model": "hsl", "channel": "lightness" },
            "points": [[0, 0.3], [0.25, 0.9], [0.75, 0.1], [1, 0.3]] }
        ] }
    ]);
    let data: Vec<u8> = (0..512 * 512u32)
        .flat_map(|index| {
            let bytes = index.to_le_bytes();
            [bytes[0], bytes[1], bytes[2], (index * 37) as u8]
        })
        .collect();
    assert_same(&effects, 512, 512, &data);
}

fn direct_curves(effect: &Value, rgb: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let dimensions = ditherette_wasm::image::ImageDimensions::new(1, 1).unwrap();
    let encoded = json!([effect]).to_string();
    let production = prod::decode_effects(&encoded).unwrap().remove(0).effect;
    let reference = spec::decode_effects(&encoded).unwrap().remove(0).effect;
    let mut production_image = prod::EffectImage {
        dimensions,
        rgb: vec![rgb],
        alpha: vec![37],
    };
    let mut reference_image = spec::EffectImage {
        dimensions,
        rgb: vec![rgb],
        alpha: vec![37],
    };
    production.apply(&mut production_image, &prod::EffectContext::default());
    spec::Effect::apply(
        &reference,
        &mut reference_image,
        &spec::EffectContext::default(),
    );
    assert_eq!(production_image.alpha, reference_image.alpha);
    (production_image.rgb[0], reference_image.rgb[0])
}

fn assert_validation_matches(effect: Value, expected_path: &str) {
    let encoded = json!([effect]).to_string();
    let production = prod::decode_effects(&encoded).unwrap().remove(0).effect;
    let reference = spec::decode_effects(&encoded).unwrap().remove(0).effect;
    let production_error = production.validate("effects.0").unwrap_err();
    let reference_error = spec::Effect::validate(&reference, "effects.0").unwrap_err();
    assert_eq!(production_error.path, expected_path);
    assert_eq!(
        (
            format!("{:?}", production_error.code),
            production_error.path,
            production_error.message
        ),
        (
            format!("{:?}", reference_error.code),
            reference_error.path,
            reference_error.message
        )
    );
}

#[test]
fn curves_validation_paths_and_registry_match_the_reference() {
    let valid = json!({ "kind": "adjust",
        "x": { "model": "srgb", "channel": "red" },
        "y": { "model": "oklch", "channel": "chroma" },
        "points": [[0, 0.5], [1, 0.5]] });
    assert_validation_matches(
        json!({ "effect": "curves", "enabled": true, "curves": vec![valid; 17] }),
        "effects.0.curves",
    );
    for (curve, path) in [
        (
            json!({ "kind": "adjust",
                "x": { "model": "hsv", "channel": "lightness" },
                "y": { "model": "srgb", "channel": "red" },
                "points": [[0, 0.5], [1, 0.5]] }),
            "effects.0.curves.0.x.channel",
        ),
        (
            json!({ "kind": "adjust",
                "x": { "model": "srgb", "channel": "red" },
                "y": { "model": "oklab", "channel": "chroma" },
                "points": [[0, 0.5], [1, 0.5]] }),
            "effects.0.curves.0.y.channel",
        ),
        (
            json!({ "kind": "remap",
                "x": { "model": "srgb", "channel": "red" },
                "y": { "model": "linear-rgb", "channel": "red" },
                "points": [[0, 0], [1, 1]] }),
            "effects.0.curves.0.y",
        ),
        (
            json!({ "kind": "remap",
                "x": { "model": "srgb", "channel": "red" },
                "y": { "model": "srgb", "channel": "red" }, "points": [[0, 0]] }),
            "effects.0.curves.0.points",
        ),
        (
            json!({ "kind": "remap",
                "x": { "model": "srgb", "channel": "red" },
                "y": { "model": "srgb", "channel": "red" },
                "points": [[0, 0], [0.0005, 1], [1, 1]] }),
            "effects.0.curves.0.points.1.0",
        ),
        (
            json!({ "kind": "adjust",
                "x": { "model": "hsl", "channel": "hue" },
                "y": { "model": "srgb", "channel": "red" },
                "points": [[0.1, 0.5], [1, 0.5]] }),
            "effects.0.curves.0.points.0.0",
        ),
        (
            json!({ "kind": "adjust",
                "x": { "model": "hsl", "channel": "hue" },
                "y": { "model": "srgb", "channel": "red" },
                "points": [[0, 0.5], [1, 0.6]] }),
            "effects.0.curves.0.points.1.1",
        ),
    ] {
        assert_validation_matches(
            json!({ "effect": "curves", "enabled": true, "curves": [curve] }),
            path,
        );
    }

    for old in [
        json!({ "effect": "curves", "enabled": true, "channel": "rgb",
            "points": [[0, 0], [1, 1]] }),
        json!({ "effect": "model-curves", "enabled": true, "model": "oklch",
            "curves": [[[0, 0], [1, 1]], [[0, 0], [1, 1]], [[0, 0], [1, 1]]] }),
        json!({ "effect": "channel-curve", "enabled": true,
            "x": { "model": "hsl", "channel": "hue" },
            "y": { "model": "oklch", "channel": "chroma" },
            "points": [[0, 0.5], [1, 0.5]] }),
    ] {
        let encoded = json!([old]).to_string();
        assert_eq!(
            prod::decode_effects(&encoded).unwrap_err().path,
            "effects.0"
        );
        assert_eq!(
            spec::decode_effects(&encoded).unwrap_err().path,
            "effects.0"
        );
    }
}

#[test]
fn curves_list_order_and_cross_model_selection_match_the_reference() {
    let selection = json!({ "effect": "curves", "enabled": true, "curves": [
        { "kind": "adjust", "x": { "model": "srgb", "channel": "red" },
          "y": { "model": "oklch", "channel": "chroma" }, "points": [[0, 1], [1, 1]] },
        { "kind": "adjust", "x": { "model": "srgb", "channel": "green" },
          "y": { "model": "cielab", "channel": "a" }, "points": [[0, 0], [1, 1]] }
    ] });
    let (actual, expected) = direct_curves(&selection, [0.25, 0.5, 0.25]);
    assert_float_bits(actual, expected);

    let low = json!({ "kind": "remap",
        "x": { "model": "srgb", "channel": "red" },
        "y": { "model": "srgb", "channel": "red" }, "points": [[0, 0.2], [1, 0.2]] });
    let high = json!({ "kind": "remap",
        "x": { "model": "srgb", "channel": "red" },
        "y": { "model": "srgb", "channel": "red" }, "points": [[0, 0.8], [1, 0.8]] });
    let forward = json!({ "effect": "curves", "enabled": true,
        "curves": [low.clone(), high.clone()] });
    let reverse = json!({ "effect": "curves", "enabled": true, "curves": [high, low] });
    let (forward, reference) = direct_curves(&forward, [0.5; 3]);
    assert_float_bits(forward, reference);
    let (reverse, reference) = direct_curves(&reverse, [0.5; 3]);
    assert_float_bits(reverse, reference);
    assert_eq!(forward[0].to_bits(), 0.8f32.to_bits());
    assert_eq!(reverse[0].to_bits(), 0.2f32.to_bits());
}

#[test]
fn curves_hue_confidence_and_carrier_overshoot_match_the_reference() {
    use ditherette_wasm::prod::effects::model::ColourModel;

    let hue = json!({ "effect": "curves", "enabled": true, "curves": [{
        "kind": "remap", "x": { "model": "hsl", "channel": "hue" },
        "y": { "model": "hsl", "channel": "hue" }, "points": [[0, 0.99], [1, 0.99]]
    }] });
    for input in [[0.4; 3], [0.51, 0.5, 0.5]] {
        let (actual, expected) = direct_curves(&hue, input);
        assert_float_bits(actual, expected);
    }
    let adjusted = ColourModel::Hsl.to_normalized(direct_curves(&hue, [0.51, 0.5, 0.5]).0);
    let expected = (0.0f32 + 0.5 * -0.01).rem_euclid(1.0);
    assert!((adjusted[0] - expected).abs() < 0.000_01);

    let linear = json!({ "effect": "curves", "enabled": true, "curves": [{
        "kind": "remap", "x": { "model": "linear-rgb", "channel": "red" },
        "y": { "model": "linear-rgb", "channel": "red" },
        "points": [[0.25, 0.2], [0.75, 0.8]]
    }] });
    let (actual, expected) = direct_curves(&linear, [-0.5, 0.5, 1.5]);
    assert_float_bits(actual, expected);
    assert!((actual[2] - 1.5).abs() < 0.000_01);
}

#[test]
fn curves_closed_hue_seam_matches_the_reference() {
    let points = [[0.0, 0.5], [0.25, 0.9], [0.5, 0.2], [0.75, 0.7], [1.0, 0.5]];
    let production = prod::curves::PeriodicSpline::new(&points);
    let reference = spec::curves::PeriodicSpline::new(&points);
    for x in [-0.25, 0.0, 0.000_01, 0.25, 0.75, 0.999_99, 1.0, 1.25] {
        assert_eq!(
            production.eval(x).to_bits(),
            reference.eval(x).to_bits(),
            "{x}"
        );
    }
    assert!((production.eval(0.000_01) - production.eval(0.999_99)).abs() < 0.000_1);
}

#[test]
fn empty_curves_and_neutral_adjustments_are_exact_no_ops() {
    let carrier = [-64.0, 1.25, 64.0];
    for effect in [
        json!({ "effect": "curves", "enabled": true, "curves": [] }),
        json!({ "effect": "curves", "enabled": true, "curves": [{
            "kind": "adjust", "x": { "model": "hsl", "channel": "hue" },
            "y": { "model": "oklch", "channel": "chroma" },
            "points": [[0, 0.5], [0.5, 0.5], [1, 0.5]]
        }] }),
    ] {
        let (actual, expected) = direct_curves(&effect, carrier);
        assert_float_bits(actual, expected);
        assert_float_bits(actual, carrier);
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
