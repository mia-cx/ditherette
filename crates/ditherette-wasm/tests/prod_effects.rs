//! Production effects must equal the frozen reference byte-for-byte.

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

fn random_effect(rng: &mut Rng) -> Value {
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
            space: None,
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
            space: None,
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
        for _ in 0..40 {
            let count = rng.next() % 6;
            let effects: Vec<Value> = (0..count).map(|_| random_effect(&mut rng)).collect();
            assert_same(&Value::Array(effects), width, height, &data);
        }
    }
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
                    let step = prod::decode_effects(&json!([random_effect(&mut rng)]).to_string())
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
