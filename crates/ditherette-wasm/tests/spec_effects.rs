use ditherette_wasm::{
    image::contracts::{PaletteEntry, Rgba8Image},
    spec::{
        contract::{
            error::{DitheretteError, ErrorCode},
            request::*,
        },
        coverage,
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

fn as_levels(step: &EffectStep) -> Levels {
    match &step.effect {
        BuiltinEffect::Levels(levels) => *levels,
        other => panic!("expected levels, got {other:?}"),
    }
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
    let (a, b) = (as_levels(&chain[0]), as_levels(&chain[1]));
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
        (ErrorCode::InvalidSettings, "effects.1")
    );
    assert!(unknown.message.contains("levles"), "{}", unknown.message);
    let mut typo = levels((0.0, 1.0), 1.0, (0.0, 1.0));
    typo["gama"] = json!(1);
    assert_eq!(error(json!([typo])).path, "effects.0");
    let mut no_toggle = levels((0.0, 1.0), 1.0, (0.0, 1.0));
    no_toggle.as_object_mut().unwrap().remove("enabled");
    assert_eq!(error(json!([no_toggle])).path, "effects.0");
    assert_eq!(error(json!({ "effect": "levels" })).path, "effects");

    let invalid = |value: serde_json::Value, disabled: bool| {
        let mut chain = steps(json!([levels((0.0, 1.0), 1.0, (0.0, 1.0)), value]));
        chain[1].enabled = !disabled;
        run(&data, &chain).unwrap_err()
    };
    for disabled in [false, true] {
        assert_eq!(
            invalid(levels((0.5, 0.5), 1.0, (0.0, 1.0)), disabled).path,
            "effects.1.input"
        );
        assert_eq!(
            invalid(levels((0.0, 1.0), 20.0, (0.0, 1.0)), disabled).path,
            "effects.1.gamma"
        );
        assert_eq!(
            invalid(levels((0.0, 1.0), 1.0, (0.0, 1.5)), disabled).path,
            "effects.1.output.white"
        );
    }
    let nan = vec![Step {
        enabled: true,
        effect: BuiltinEffect::Levels(Levels {
            gamma: f32::NAN,
            ..as_levels(&steps(json!([levels((0.0, 1.0), 1.0, (0.0, 1.0))]))[0])
        }),
    }];
    assert_eq!(run(&data, &nan).unwrap_err().path, "effects.0.gamma");
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
fn process_v2_equals_effects_then_coverage_then_v1_process_for_every_dither_family() {
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
        let resized = coverage::resize(ResizeRequest {
            version: 1,
            source: Source {
                width: 16,
                height: 16,
                data: effected.data(),
            },
            output: recipe.output,
        })
        .unwrap();
        let staged = pipeline::process(ProcessRequest {
            source: Source {
                width: 7,
                height: 5,
                data: resized.data(),
            },
            palette: &PALETTE,
            recipe: RecipeV1 {
                output: Output {
                    resize: ResizePolicy::Nearest {
                        anchor: Anchor::Center,
                    },
                    ..recipe.output
                },
                ..recipe.terminal()
            },
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
        let covered = coverage::resize(ResizeRequest {
            version: 1,
            source: source(&data),
            output: recipe.output,
        })
        .unwrap();
        let staged_plain = pipeline::process(ProcessRequest {
            source: Source {
                width: 7,
                height: 5,
                data: covered.data(),
            },
            palette: &PALETTE,
            recipe: RecipeV1 {
                output: Output {
                    resize: ResizePolicy::Nearest {
                        anchor: Anchor::Center,
                    },
                    ..recipe.output
                },
                ..recipe.terminal()
            },
        })
        .unwrap();
        assert_eq!(plain, staged_plain, "{dither}");
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
    assert_eq!(error.path, "recipe.effects.0.gamma");
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
    assert_eq!(decode.path, "recipe.effects.0");
}

#[test]
fn chains_are_capped_and_see_only_the_retained_palette() {
    let data = ramp();
    let neutral = levels((0.0, 1.0), 1.0, (0.0, 1.0));
    let long = steps(serde_json::Value::Array(vec![neutral.clone(); 64]));
    assert_eq!(run(&data, &long).unwrap().data(), data.as_slice());
    let too_long = steps(serde_json::Value::Array(vec![neutral; 65]));
    assert_eq!(run(&data, &too_long).unwrap_err().path, "effects");

    // Quantization keeps the first 256 entries, so a colour at index 256 is not context.
    let mut palette = vec![PaletteEntry::Transparent {}; 256];
    palette.push(PaletteEntry::Color { rgb: [1, 2, 3] });
    let context = EffectContext {
        palette: &palette,
        space: Some(WorkingSpace::Oklab),
    };
    assert_eq!(context.colors().count(), 0);
    let probe = vec![Step {
        enabled: true,
        effect: PaletteProbe,
    }];
    let dimensions = run(&data, &[]).unwrap().dimensions();
    let view = ditherette_wasm::image::ImageView::packed(&data, dimensions).unwrap();
    let mut image = EffectImage::from_rgba8(view);
    assert_eq!(
        apply_chain(&mut image, &probe, &context).unwrap_err().path,
        "context.palette"
    );
}

fn channel(model: &str, channel: &str) -> serde_json::Value {
    json!({ "model": model, "channel": channel })
}

fn curve(
    kind: &str,
    x: (&str, &str),
    y: (&str, &str),
    points: serde_json::Value,
) -> serde_json::Value {
    json!({
        "kind": kind,
        "x": channel(x.0, x.1),
        "y": channel(y.0, y.1),
        "points": points,
    })
}

fn curves(curves: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "effect": "curves", "enabled": true, "curves": curves })
}

fn apply_curves(effect: &serde_json::Value, rgb: [f32; 3]) -> ([f32; 3], u8) {
    use ditherette_wasm::image::ImageDimensions;

    let step = steps(json!([effect])).pop().unwrap();
    let BuiltinEffect::Curves(effect) = step.effect else {
        unreachable!()
    };
    let mut image = EffectImage {
        dimensions: ImageDimensions::new(1, 1).unwrap(),
        rgb: vec![rgb],
        alpha: vec![37],
    };
    effect.apply(&mut image, &EffectContext::default());
    (image.rgb[0], image.alpha[0])
}

#[test]
fn curves_reject_every_old_json_shape() {
    let old = [
        json!({
            "effect": "curves", "enabled": true, "channel": "rgb",
            "points": [[0, 0], [1, 1]],
        }),
        json!({
            "effect": "model-curves", "enabled": true, "model": "oklch",
            "curves": [
                [[0, 0], [1, 1]],
                [[0, 0], [1, 1]],
                [[0, 0], [1, 1]],
            ],
        }),
        json!({
            "effect": "channel-curve", "enabled": true,
            "x": channel("hsl", "hue"),
            "y": channel("oklch", "chroma"),
            "points": [[0, 0.5], [1, 0.5]],
        }),
    ];
    for effect in old {
        assert_eq!(
            decode_effects(&json!([effect]).to_string())
                .unwrap_err()
                .path,
            "effects.0"
        );
    }
}

#[test]
fn curves_validate_limits_channels_points_and_hue_seams() {
    let data = ramp();
    let valid = curve(
        "adjust",
        ("srgb", "red"),
        ("oklch", "chroma"),
        json!([[0, 0.5], [1, 0.5]]),
    );
    let too_many = curves(vec![valid; 17]);
    assert_eq!(
        run(&data, &steps(json!([too_many]))).unwrap_err().path,
        "effects.0.curves"
    );

    let cases = [
        (
            curve(
                "adjust",
                ("hsv", "lightness"),
                ("srgb", "red"),
                json!([[0, 0.5], [1, 0.5]]),
            ),
            "effects.0.curves.0.x.channel",
        ),
        (
            curve(
                "adjust",
                ("srgb", "red"),
                ("oklab", "chroma"),
                json!([[0, 0.5], [1, 0.5]]),
            ),
            "effects.0.curves.0.y.channel",
        ),
        (
            curve(
                "remap",
                ("srgb", "red"),
                ("linear-rgb", "red"),
                json!([[0, 0], [1, 1]]),
            ),
            "effects.0.curves.0.y",
        ),
        (
            curve("remap", ("srgb", "red"), ("srgb", "red"), json!([[0, 0]])),
            "effects.0.curves.0.points",
        ),
        (
            curve(
                "remap",
                ("srgb", "red"),
                ("srgb", "red"),
                json!([[0, 0], [0.0005, 1], [1, 1]]),
            ),
            "effects.0.curves.0.points.1.0",
        ),
        (
            curve(
                "adjust",
                ("srgb", "red"),
                ("hsl", "lightness"),
                json!([[0, 0.5], [1, 1.5]]),
            ),
            "effects.0.curves.0.points.1.1",
        ),
        (
            curve(
                "adjust",
                ("hsl", "hue"),
                ("srgb", "red"),
                json!([[0.1, 0.5], [1, 0.5]]),
            ),
            "effects.0.curves.0.points.0.0",
        ),
        (
            curve(
                "adjust",
                ("hsl", "hue"),
                ("srgb", "red"),
                json!([[0, 0.5], [0.9, 0.5]]),
            ),
            "effects.0.curves.0.points.1.0",
        ),
        (
            curve(
                "adjust",
                ("hsl", "hue"),
                ("srgb", "red"),
                json!([[0, 0.5], [1, 0.6]]),
            ),
            "effects.0.curves.0.points.1.1",
        ),
    ];
    for (entry, path) in cases {
        let error = run(&data, &steps(json!([curves(vec![entry])]))).unwrap_err();
        assert_eq!(error.path, path);
    }
}

#[test]
fn empty_curves_and_neutral_adjustments_preserve_the_carrier() {
    use ditherette_wasm::image::ImageDimensions;
    use ditherette_wasm::spec::effects::curves::Curves;

    let carrier = [-64.0, 1.25, 64.0];
    let effects = [
        curves(vec![]),
        curves(vec![curve(
            "adjust",
            ("hsl", "hue"),
            ("oklch", "chroma"),
            json!([[0, 0.5], [0.5, 0.5], [1, 0.5]]),
        )]),
    ];
    for effect in effects {
        let step = steps(json!([effect])).pop().unwrap();
        let BuiltinEffect::Curves(effect) = step.effect else {
            unreachable!()
        };
        let mut image = EffectImage {
            dimensions: ImageDimensions::new(1, 1).unwrap(),
            rgb: vec![carrier],
            alpha: vec![17],
        };
        effect.apply(&mut image, &EffectContext::default());
        assert_eq!(image.rgb[0].map(f32::to_bits), carrier.map(f32::to_bits));
        assert_eq!(image.alpha, [17]);
    }

    let decoded = steps(json!([curves(vec![])]));
    let BuiltinEffect::Curves(Curves { curves }) = &decoded[0].effect else {
        unreachable!()
    };
    assert!(curves.is_empty());
}

#[test]
fn periodic_spline_wraps_and_hits_knots_without_overshoot() {
    use ditherette_wasm::spec::effects::curves::PeriodicSpline;

    let spline =
        PeriodicSpline::new(&[[0.0, 0.5], [0.25, 0.9], [0.5, 0.2], [0.75, 0.7], [1.0, 0.5]]);
    for [x, y] in [[0.0, 0.5], [0.25, 0.9], [0.5, 0.2], [0.75, 0.7], [1.0, 0.5]] {
        assert_eq!(spline.eval(x).to_bits(), y.to_bits(), "knot {x}");
    }
    assert_eq!(spline.eval(-0.25).to_bits(), spline.eval(0.75).to_bits());
    assert_eq!(spline.eval(1.25).to_bits(), spline.eval(0.25).to_bits());
    assert!((spline.eval(0.000_01) - spline.eval(0.999_99)).abs() < 0.000_1);
    for index in 0..1000 {
        let value = spline.eval(index as f32 / 1000.0);
        assert!((0.2..=0.9).contains(&value), "{index}: {value}");
    }
}

#[test]
fn ordered_curves_select_from_source_and_edit_the_accumulated_result() {
    let selection = curves(vec![
        curve(
            "adjust",
            ("srgb", "red"),
            ("srgb", "green"),
            json!([[0, 1], [1, 1]]),
        ),
        curve(
            "adjust",
            ("srgb", "green"),
            ("srgb", "blue"),
            json!([[0, 0], [1, 1]]),
        ),
    ]);
    let output = apply_curves(&selection, [0.25, 0.5, 0.25]).0;
    assert_eq!(
        output.map(f32::to_bits),
        [0.25, 1.0, 0.25].map(f32::to_bits)
    );

    let low = curve(
        "remap",
        ("srgb", "red"),
        ("srgb", "red"),
        json!([[0, 0.2], [1, 0.2]]),
    );
    let high = curve(
        "remap",
        ("srgb", "red"),
        ("srgb", "red"),
        json!([[0, 0.8], [1, 0.8]]),
    );
    assert_eq!(
        apply_curves(&curves(vec![low.clone(), high.clone()]), [0.5; 3]).0[0].to_bits(),
        0.8f32.to_bits()
    );
    assert_eq!(
        apply_curves(&curves(vec![high, low]), [0.5; 3]).0[0].to_bits(),
        0.2f32.to_bits()
    );
}

#[test]
fn remaps_preserve_model_curve_hue_confidence_and_carrier_rules() {
    use ditherette_wasm::spec::effects::model::ColourModel;

    let hue = curves(vec![curve(
        "remap",
        ("hsl", "hue"),
        ("hsl", "hue"),
        json!([[0, 0.99], [1, 0.99]]),
    )]);
    let input = ColourModel::Hsl.from_normalized([0.01, 1.0, 0.5]);
    let wrapped = ColourModel::Hsl.to_normalized(apply_curves(&hue, input).0);
    assert!((wrapped[0] - 0.99).abs() < 0.000_01, "{}", wrapped[0]);

    let grey = [0.4; 3];
    assert_eq!(
        apply_curves(&hue, grey).0.map(f32::to_bits),
        grey.map(f32::to_bits)
    );
    let half = [0.51, 0.5, 0.5];
    let adjusted = ColourModel::Hsl.to_normalized(apply_curves(&hue, half).0);
    let expected = (0.0f32 + 0.5 * -0.01).rem_euclid(1.0);
    assert!((adjusted[0] - expected).abs() < 0.000_01, "{}", adjusted[0]);

    let linear = curves(vec![curve(
        "remap",
        ("linear-rgb", "red"),
        ("linear-rgb", "red"),
        json!([[0.25, 0.2], [0.75, 0.8]]),
    )]);
    let output = apply_curves(&linear, [-0.5, 0.5, 1.5]).0;
    let normalized = ColourModel::LinearRgb.to_normalized(output);
    assert!((normalized[0] - 0.2).abs() < f32::EPSILON);
    assert!((output[2] - 1.5).abs() < 0.000_01);
}

#[test]
fn adjustments_use_literal_turn_gain_and_offset_formulas() {
    use ditherette_wasm::spec::effects::model::ColourModel;

    let ramp = json!([[0, 0], [1, 1]]);
    let hue = curves(vec![curve(
        "adjust",
        ("srgb", "red"),
        ("hsl", "hue"),
        ramp.clone(),
    )]);
    let input = ColourModel::Hsl.from_normalized([0.0, 1.0, 0.5]);
    let (output, alpha) = apply_curves(&hue, input);
    assert_eq!(
        ColourModel::Hsl.to_normalized(output)[0].to_bits(),
        0.5f32.to_bits()
    );
    assert_eq!(alpha, 37);

    let gain = curves(vec![curve(
        "adjust",
        ("srgb", "red"),
        ("hsl", "saturation"),
        ramp.clone(),
    )]);
    let input = ColourModel::Hsl.from_normalized([0.5, 1.0, 0.5]);
    assert_eq!(
        ColourModel::Hsl.to_normalized(apply_curves(&gain, input).0)[1].to_bits(),
        0.0f32.to_bits()
    );

    let offset = curves(vec![curve(
        "adjust",
        ("srgb", "blue"),
        ("hsl", "lightness"),
        ramp,
    )]);
    let input = ColourModel::Hsl.from_normalized([0.0, 1.0, 0.5]);
    assert_eq!(
        ColourModel::Hsl.to_normalized(apply_curves(&offset, input).0)[2].to_bits(),
        0.0f32.to_bits()
    );
}

#[test]
fn hue_dependent_adjustments_use_source_and_current_confidence() {
    use ditherette_wasm::spec::effects::model::ColourModel;

    let hue_input = curves(vec![curve(
        "adjust",
        ("hsl", "hue"),
        ("srgb", "blue"),
        json!([[0, 1], [0.5, 0.5], [1, 1]]),
    )]);
    let grey = [0.5; 3];
    assert_eq!(
        apply_curves(&hue_input, grey).0.map(f32::to_bits),
        grey.map(f32::to_bits)
    );
    let half = apply_curves(&hue_input, [0.51, 0.5, 0.5]).0;
    assert!((half[2] - 0.75).abs() < 0.000_01, "{}", half[2]);

    let hue_output = curves(vec![curve(
        "adjust",
        ("srgb", "red"),
        ("hsl", "hue"),
        json!([[0, 1], [1, 1]]),
    )]);
    assert_eq!(
        apply_curves(&hue_output, grey).0.map(f32::to_bits),
        grey.map(f32::to_bits)
    );
    let adjusted = ColourModel::Hsl.to_normalized(apply_curves(&hue_output, [0.51, 0.5, 0.5]).0);
    assert!((adjusted[0] - 0.25).abs() < 0.000_01, "{}", adjusted[0]);

    for model in ["oklch", "cielch"] {
        let x = curves(vec![curve(
            "adjust",
            (model, "hue"),
            ("srgb", "blue"),
            json!([[0, 1], [0.5, 1], [1, 1]]),
        )]);
        let y = curves(vec![curve(
            "adjust",
            ("srgb", "red"),
            (model, "hue"),
            json!([[0, 1], [1, 1]]),
        )]);
        for effect in [x, y] {
            assert_eq!(
                apply_curves(&effect, grey).0.map(f32::to_bits),
                grey.map(f32::to_bits),
                "{model}"
            );
        }
    }
}

#[test]
fn model_conversion_formulas_remain_literal() {
    use ditherette_wasm::spec::effects::model::ColourModel;

    assert_eq!(
        ColourModel::Hsl.to_normalized([1.0, 0.0, 0.0]),
        [0.0, 1.0, 0.5]
    );
    assert_eq!(
        ColourModel::Hsv.to_normalized([1.0, 0.0, 0.0]),
        [0.0, 1.0, 1.0]
    );
    let ycbcr = ColourModel::Ycbcr.to_normalized([1.0, 0.0, 0.0]);
    assert_eq!(ycbcr[0].to_bits(), 0.299f32.to_bits());
    assert_eq!(ycbcr[1].to_bits(), (0.5 - 0.299f32 / 1.772).to_bits());
    assert_eq!(ycbcr[2].to_bits(), (0.5 + 0.701f32 / 1.402).to_bits());
    for model in [ColourModel::Oklch, ColourModel::Cielch] {
        let grey = [0.5; 3];
        assert_eq!(model.hue_weight(grey, model.to_normalized(grey)), 0.0);
    }
}
