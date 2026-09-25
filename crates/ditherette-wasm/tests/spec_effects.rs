use ditherette_wasm::{
    image::contracts::{PaletteEntry, Rgba8Image},
    spec::{
        contract::{
            error::{DitheretteError, ErrorCode},
            request::*,
        },
        effects::{
            apply_chain, apply_effects, decode_effects, decode_recipe_v2, levels::Levels, process,
            BuiltinEffect, Effect, EffectContext, EffectImage, EffectStep, EffectsRequest, Needs,
            ProcessRequestV2, RecipeV2, Step,
        },
        pipeline,
    },
};
use serde_json::json;

const PALETTE: [PaletteEntry; 4] = [
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Transparent {},
    PaletteEntry::Color {
        rgb: [200, 120, 40],
    },
    PaletteEntry::Color {
        rgb: [255, 255, 255],
    },
];

/// Every byte value on each channel, with varied alpha including zero.
fn ramp() -> Vec<u8> {
    (0..=255u8)
        .flat_map(|value| {
            [
                value,
                255 - value,
                value.wrapping_mul(7),
                value.wrapping_mul(3),
            ]
        })
        .collect()
}

fn source(data: &[u8]) -> Source<'_> {
    Source {
        width: 16,
        height: (data.len() / 64) as u32,
        data,
    }
}

fn steps(value: serde_json::Value) -> Vec<EffectStep> {
    decode_effects(&value.to_string()).expect("fixture effects decode")
}

fn run(data: &[u8], effects: &[EffectStep]) -> Result<Rgba8Image, DitheretteError> {
    apply_effects(EffectsRequest {
        version: 1,
        source: source(data),
        effects,
        context: EffectContext::default(),
    })
}

fn levels(input: (f32, f32), gamma: f32, output: (f32, f32)) -> serde_json::Value {
    json!({
        "effect": "levels", "enabled": true, "channel": "rgb",
        "input": { "black": input.0, "white": input.1 },
        "gamma": gamma,
        "output": { "black": output.0, "white": output.1 },
    })
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[test]
fn empty_and_disabled_chains_return_the_source_bytes() {
    let data = ramp();
    assert_eq!(run(&data, &[]).unwrap().data(), data.as_slice());
    let mut disabled = steps(json!([levels((0.2, 0.4), 3.0, (1.0, 0.0))]));
    disabled[0].enabled = false;
    assert_eq!(run(&data, &disabled).unwrap().data(), data.as_slice());
}

#[test]
fn neutral_levels_is_an_exact_identity() {
    let data = ramp();
    let neutral = steps(json!([levels((0.0, 1.0), 1.0, (0.0, 1.0))]));
    assert_eq!(run(&data, &neutral).unwrap().data(), data.as_slice());
}

#[test]
fn levels_follows_its_documented_formula_and_channel() {
    let data = ramp();
    let chain = steps(json!([{
        "effect": "levels", "enabled": true, "channel": "green",
        "input": { "black": 0.2, "white": 0.8 }, "gamma": 2.0,
        "output": { "black": 1.0, "white": 0.1 },
    }]));
    let output = run(&data, &chain).unwrap();
    for (input, output) in data.chunks(4).zip(output.data().chunks(4)) {
        let t = ((input[1] as f32 / 255.0 - 0.2f32) / (0.8f32 - 0.2)).clamp(0.0, 1.0);
        let expected = 1.0 + (0.1f32 - 1.0) * t.powf(1.0 / 2.0);
        assert_eq!(output, [input[0], byte(expected), input[2], input[3]]);
    }
}

#[test]
fn order_is_caller_order_and_the_two_orders_differ() {
    let data = ramp();
    let expand = levels((0.0, 0.5), 1.0, (0.0, 1.0));
    let compress = levels((0.0, 1.0), 1.0, (0.25, 0.75));
    let forward = run(&data, &steps(json!([expand, compress]))).unwrap();
    let reverse = run(&data, &steps(json!([compress, expand]))).unwrap();
    assert_ne!(forward, reverse);
    for (input, output) in data.chunks(4).zip(forward.data().chunks(4)) {
        let map = |value: u8| {
            let expanded = ((value as f32 / 255.0) / 0.5).clamp(0.0, 1.0);
            byte(0.25 + 0.5 * expanded)
        };
        assert_eq!(
            output,
            [map(input[0]), map(input[1]), map(input[2]), input[3]]
        );
    }
}

fn forward_first(data: &[u8], effect: &serde_json::Value) -> Rgba8Image {
    run(data, &steps(json!([effect]))).unwrap()
}

#[test]
fn repeated_instances_keep_independent_arguments() {
    let data = ramp();
    let first = levels((0.1, 0.9), 0.5, (0.0, 1.0));
    let second = levels((0.0, 1.0), 2.0, (0.2, 0.8));
    let chain = steps(json!([first, second]));
    let output = run(&data, &chain).unwrap();
    let [BuiltinEffect::Levels(a), BuiltinEffect::Levels(b)] = [&chain[0].effect, &chain[1].effect];
    for (input, output) in data.chunks(4).zip(output.data().chunks(4)) {
        let map = |value: u8| byte(b.map(a.map(value as f32 / 255.0)));
        assert_eq!(
            output,
            [map(input[0]), map(input[1]), map(input[2]), input[3]]
        );
    }
}

#[test]
fn intermediate_values_stay_continuous_until_the_boundary() {
    let data = ramp();
    let halve = levels((0.0, 1.0), 1.0, (0.0, 0.5));
    let double = levels((0.0, 0.5), 1.0, (0.0, 1.0));
    assert_eq!(
        run(&data, &steps(json!([halve, double]))).unwrap().data(),
        data.as_slice()
    );
    let halved = forward_first(&data, &halve);
    let staged = run(halved.data(), &steps(json!([double]))).unwrap();
    assert_ne!(
        staged.data(),
        data.as_slice(),
        "byte staging loses odd values"
    );
}

#[test]
fn recipes_round_trip_and_replay_deterministically() {
    let data = ramp();
    let chain = steps(json!([
        levels((0.05, 0.95), 1.4, (0.0, 1.0)),
        levels((0.0, 1.0), 0.7, (0.1, 0.9)),
    ]));
    let serialized = serde_json::to_string(&chain).unwrap();
    assert_eq!(decode_effects(&serialized).unwrap(), chain);
    assert_eq!(run(&data, &chain).unwrap(), run(&data, &chain).unwrap());
}

/// A caller-defined effect, used without touching the executor.
struct Invert;

impl Effect for Invert {
    fn validate(&self, _path: &str) -> Result<(), DitheretteError> {
        Ok(())
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        for rgb in &mut image.rgb {
            *rgb = rgb.map(|value| 1.0 - value);
        }
    }
}

/// Needs a palette and records its visible colour count in red.
struct PaletteProbe;

impl Effect for PaletteProbe {
    fn validate(&self, _path: &str) -> Result<(), DitheretteError> {
        Ok(())
    }

    fn needs(&self) -> Needs {
        Needs {
            palette: true,
            space: true,
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        let count = context.colors().count() as f32;
        for rgb in &mut image.rgb {
            rgb[0] = count / 255.0;
        }
    }
}

#[test]
fn caller_defined_effects_compose_with_builtins_through_the_same_executor() {
    let data = ramp();
    let dimensions = run(&data, &[]).unwrap().dimensions();
    let view = ditherette_wasm::image::ImageView::packed(&data, dimensions).unwrap();
    let builtin = steps(json!([levels((0.0, 1.0), 1.0, (0.0, 0.5))])).remove(0);
    let chain: Vec<Step<Box<dyn Effect>>> = vec![
        Step {
            enabled: true,
            effect: Box::new(builtin.effect),
        },
        Step {
            enabled: true,
            effect: Box::new(Invert),
        },
        Step {
            enabled: false,
            effect: Box::new(PaletteProbe),
        },
    ];
    let mut image = EffectImage::from_rgba8(view);
    apply_chain(&mut image, &chain, &EffectContext::default()).unwrap();
    for (input, output) in data.chunks(4).zip(image.to_rgba8().data().chunks(4)) {
        let map = |value: u8| byte(1.0 - 0.5 * (value as f32 / 255.0));
        assert_eq!(
            output,
            [map(input[0]), map(input[1]), map(input[2]), input[3]]
        );
    }

    let probe = vec![Step {
        enabled: true,
        effect: PaletteProbe,
    }];
    let mut image = EffectImage::from_rgba8(view);
    let error = apply_chain(&mut image, &probe, &EffectContext::default()).unwrap_err();
    assert_eq!(error.path, "context.palette");
    let context = EffectContext {
        palette: &PALETTE,
        space: None,
    };
    assert_eq!(
        apply_chain(&mut image, &probe, &context).unwrap_err().path,
        "context.space"
    );
    let context = EffectContext {
        palette: &PALETTE,
        space: Some(WorkingSpace::Oklab),
    };
    apply_chain(&mut image, &probe, &context).unwrap();
    assert!(image.to_rgba8().data().chunks(4).all(|pixel| pixel[0] == 3));
}

#[test]
fn decoding_and_validation_name_the_failing_step() {
    let data = ramp();
    let error = |value: serde_json::Value| decode_effects(&value.to_string()).unwrap_err();
    let unknown = error(
        json!([levels((0.0, 1.0), 1.0, (0.0, 1.0)), { "effect": "levles", "enabled": true }]),
    );
    assert_eq!(
        (unknown.code, unknown.path.as_str()),
        (ErrorCode::InvalidSettings, "effects[1]")
    );
    assert!(unknown.message.contains("levles"), "{}", unknown.message);
    let mut typo = levels((0.0, 1.0), 1.0, (0.0, 1.0));
    typo["gama"] = json!(1);
    assert_eq!(error(json!([typo])).path, "effects[0]");
    let mut no_toggle = levels((0.0, 1.0), 1.0, (0.0, 1.0));
    no_toggle.as_object_mut().unwrap().remove("enabled");
    assert_eq!(error(json!([no_toggle])).path, "effects[0]");
    assert_eq!(error(json!({ "effect": "levels" })).path, "effects");

    let invalid = |value: serde_json::Value, disabled: bool| {
        let mut chain = steps(json!([levels((0.0, 1.0), 1.0, (0.0, 1.0)), value]));
        chain[1].enabled = !disabled;
        run(&data, &chain).unwrap_err()
    };
    for disabled in [false, true] {
        assert_eq!(
            invalid(levels((0.5, 0.5), 1.0, (0.0, 1.0)), disabled).path,
            "effects[1].input"
        );
        assert_eq!(
            invalid(levels((0.0, 1.0), 20.0, (0.0, 1.0)), disabled).path,
            "effects[1].gamma"
        );
        assert_eq!(
            invalid(levels((0.0, 1.0), 1.0, (0.0, 1.5)), disabled).path,
            "effects[1].output.white"
        );
    }
    let nan = vec![Step {
        enabled: true,
        effect: BuiltinEffect::Levels(Levels {
            gamma: f32::NAN,
            ..match &steps(json!([levels((0.0, 1.0), 1.0, (0.0, 1.0))]))[0].effect {
                BuiltinEffect::Levels(levels) => *levels,
            }
        }),
    }];
    assert_eq!(run(&data, &nan).unwrap_err().path, "effects[0].gamma");
    let version = apply_effects(EffectsRequest {
        version: 2,
        source: source(&data),
        effects: &[],
        context: EffectContext::default(),
    })
    .unwrap_err();
    assert_eq!(
        (version.code, version.path.as_str()),
        (ErrorCode::InvalidRequest, "version")
    );
}

fn recipe(effects: serde_json::Value, dither: serde_json::Value) -> RecipeV2 {
    decode_recipe_v2(
        &json!({
            "version": 2,
            "effects": effects,
            "output": { "width": 7, "height": 5, "resize": { "algorithm": "area" } },
            "alpha": { "mode": "preserve", "threshold": 127.5 },
            "match": "oklab-euclidean",
            "dither": dither,
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn process_v2_equals_effects_then_v1_process_for_every_dither_family() {
    let data = ramp();
    let effects = json!([
        levels((0.1, 0.8), 1.3, (0.0, 1.0)),
        { "effect": "levels", "enabled": true, "channel": "blue",
          "input": { "black": 0.0, "white": 1.0 }, "gamma": 1.0,
          "output": { "black": 0.3, "white": 0.6 } },
    ]);
    let dithers = [
        json!({ "family": "none" }),
        json!({ "family": "separable", "perturb": {
            "field": { "algorithm": "bayer", "size": "4" }, "space": "oklab",
            "strength": 0.6, "placement": { "mode": "everywhere" } } }),
        json!({ "family": "diffusion", "kernel": "floyd-steinberg", "strength": 1.0,
            "placement": { "mode": "everywhere" }, "serpentine": true, "feedback": "matching" }),
        json!({ "family": "yliluoma", "size": "4", "placement": { "mode": "everywhere" } }),
    ];
    for dither in dithers {
        let recipe = recipe(effects.clone(), dither.clone());
        let composed = process(ProcessRequestV2 {
            source: source(&data),
            palette: &PALETTE,
            recipe: &recipe,
        })
        .unwrap();
        let effected = run(&data, &recipe.effects).unwrap();
        let staged = pipeline::process(ProcessRequest {
            source: Source {
                width: 16,
                height: 16,
                data: effected.data(),
            },
            palette: &PALETTE,
            recipe: recipe.terminal(),
        })
        .unwrap();
        assert_eq!(composed, staged, "{dither}");
        let palette_len = composed.palette.rgba.len() / 4;
        assert!(composed
            .indices
            .data()
            .iter()
            .all(|&index| (index as usize) < palette_len));

        let untreated = recipe_with(&recipe, Vec::new());
        let plain = process(ProcessRequestV2 {
            source: source(&data),
            palette: &PALETTE,
            recipe: &untreated,
        })
        .unwrap();
        let v1 = pipeline::process(ProcessRequest {
            source: source(&data),
            palette: &PALETTE,
            recipe: recipe.terminal(),
        })
        .unwrap();
        assert_eq!(plain, v1, "{dither}");
    }
}

fn recipe_with(recipe: &RecipeV2, effects: Vec<EffectStep>) -> RecipeV2 {
    RecipeV2 {
        effects,
        ..recipe.clone()
    }
}

#[test]
fn process_v2_validates_effects_under_recipe_before_any_work() {
    let data = ramp();
    let base = recipe(json!([]), json!({ "family": "none" }));
    let bad = recipe_with(&base, steps(json!([levels((0.0, 1.0), 0.0, (0.0, 1.0))])));
    let error = process(ProcessRequestV2 {
        source: source(&data),
        palette: &PALETTE,
        recipe: &bad,
    })
    .unwrap_err();
    assert_eq!(error.path, "recipe.effects[0].gamma");
    let v1 = RecipeV2 {
        version: 1,
        ..base.clone()
    };
    let error = process(ProcessRequestV2 {
        source: source(&data),
        palette: &PALETTE,
        recipe: &v1,
    })
    .unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidRequest, "recipe.version")
    );
    let decode = decode_recipe_v2(
        &json!({ "version": 2, "effects": [{ "effect": "blur", "enabled": true }] }).to_string(),
    )
    .unwrap_err();
    assert_eq!(decode.path, "recipe.effects[0]");
}
