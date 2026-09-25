//! Grading effects: formulas, neutral no-ops, curve shape, and validation paths.

use ditherette_wasm::{
    image::{contracts::Rgba8Image, ImageDimensions},
    spec::{
        color::{linear_to_srgb_unit, srgb_unit_to_linear},
        contract::{error::DitheretteError, request::Source},
        effects::{
            apply_effects,
            curves::Spline,
            decode_effects,
            space::{linear_to_oklab, to_linear},
            Effect, EffectContext, EffectImage, EffectsRequest,
        },
    },
};
use serde_json::{json, Value};

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

fn run(data: &[u8], effects: Value) -> Result<Rgba8Image, DitheretteError> {
    let steps = decode_effects(&effects.to_string()).expect("fixture decodes");
    apply_effects(EffectsRequest {
        version: 1,
        source: Source {
            width: 16,
            height: (data.len() / 64) as u32,
            data,
        },
        effects: &steps,
        context: EffectContext::default(),
    })
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn neutral() -> [Value; 5] {
    [
        json!({ "effect": "curves", "enabled": true, "channel": "rgb", "points": [[0, 0], [1, 1]] }),
        json!({ "effect": "brightness-contrast", "enabled": true, "brightness": 0, "contrast": 0 }),
        json!({ "effect": "exposure", "enabled": true, "stops": 0 }),
        json!({ "effect": "white-balance", "enabled": true, "temperature": 0, "tint": 0 }),
        json!({ "effect": "hue-saturation", "enabled": true, "hue": 0, "saturation": 0, "lightness": 0 }),
    ]
}

#[test]
fn neutral_arguments_are_exact_no_ops() {
    let data = ramp();
    for effect in neutral() {
        assert_eq!(
            run(&data, json!([effect])).unwrap().data(),
            data,
            "{effect}"
        );
    }
    // Unclipped carrier values survive too, except curves, which clamps to its end points.
    let carrier = EffectImage {
        dimensions: ImageDimensions::new(3, 1).unwrap(),
        rgb: vec![[1.3, -0.2, 0.5], [0.0, 1.0, 2.0], [-1.0, 0.25, 0.75]],
        alpha: vec![255, 0, 7],
    };
    for effect in &neutral()[1..] {
        let step = decode_effects(&json!([effect]).to_string())
            .unwrap()
            .remove(0);
        let mut image = carrier.clone();
        step.effect.apply(&mut image, &EffectContext::default());
        assert_eq!(image, carrier, "{effect}");
    }
}

#[test]
fn curves_pass_through_points_without_overshoot() {
    let spline = Spline::new(&[[0.0, 0.0], [0.3, 0.8], [0.6, 0.2], [1.0, 1.0]]);
    for [x, y] in [[0.0, 0.0], [0.3, 0.8], [0.6, 0.2], [1.0, 1.0]] {
        assert_eq!(spline.eval(x), y);
    }
    let samples: Vec<f32> = (0..=1000).map(|i| spline.eval(i as f32 / 1000.0)).collect();
    assert!(samples.iter().all(|y| (-1e-6..=1.0 + 1e-6).contains(y)));
    // Local extrema sit exactly on the knots: rising, then falling, then rising.
    let rising = |a: usize, b: usize| samples[a..b].windows(2).all(|w| w[1] >= w[0] - 1e-6);
    assert!(rising(0, 300));
    assert!(samples[300..600].windows(2).all(|w| w[1] <= w[0] + 1e-6));
    assert!(rising(600, 1001));
    assert_eq!(spline.eval(-0.5), 0.0);
    assert_eq!(spline.eval(1.5), 1.0);

    // A monotone curve stays monotone, and matches the documented Hermite form.
    let spline = Spline::new(&[[0.0, 0.1], [0.5, 0.4], [1.0, 0.95]]);
    let values: Vec<f32> = (0..=256).map(|i| spline.eval(i as f32 / 256.0)).collect();
    assert!(values.windows(2).all(|w| w[1] >= w[0]));
    let (d0, d1) = (0.3f32 / 0.5, 0.55f32 / 0.5);
    let middle = (1.5f32 + 1.5) / (1.5 / d0 + 1.5 / d1);
    let (h, s) = (0.5f32, 0.25f32);
    let c2 = (3.0 * d0 - 2.0 * d0 - middle) / h;
    let c3 = (d0 + middle - 2.0 * d0) / (h * h);
    assert_eq!(spline.eval(0.25), 0.1 + s * (d0 + s * (c2 + s * c3)));
}

#[test]
fn per_channel_effects_follow_their_formulas() {
    let data = ramp();
    let bc = run(&data, json!([{ "effect": "brightness-contrast", "enabled": true, "brightness": 0.1, "contrast": 0.5 }])).unwrap();
    let exposure = run(
        &data,
        json!([{ "effect": "exposure", "enabled": true, "stops": -1.0 }]),
    )
    .unwrap();
    let wb = run(
        &data,
        json!([{ "effect": "white-balance", "enabled": true, "temperature": 1.0, "tint": 0 }]),
    )
    .unwrap();
    for (index, pixel) in data.chunks(4).enumerate() {
        let at = |image: &Rgba8Image| image.data()[index * 4..index * 4 + 4].to_vec();
        let unit = |channel: usize| pixel[channel] as f32 / 255.0;
        let contrast = |v: f32| byte((v - 0.5) * 4f32.powf(0.5) + 0.5 + 0.1);
        assert_eq!(
            at(&bc),
            [
                contrast(unit(0)),
                contrast(unit(1)),
                contrast(unit(2)),
                pixel[3]
            ]
        );
        let halve = |v: f32| byte(linear_to_srgb_unit(srgb_unit_to_linear(v) * 0.5));
        assert_eq!(
            at(&exposure),
            [halve(unit(0)), halve(unit(1)), halve(unit(2)), pixel[3]]
        );
        let gain = |v: f32, gain: f32| byte(linear_to_srgb_unit(srgb_unit_to_linear(v) * gain));
        assert_eq!(
            at(&wb),
            [
                gain(unit(0), 2f32.powf(0.5)),
                pixel[1],
                gain(unit(2), 2f32.powf(-0.5)),
                pixel[3]
            ]
        );
    }
}

#[test]
fn hue_saturation_turns_hue_and_keeps_lightness_in_oklab() {
    let mut image = EffectImage {
        dimensions: ImageDimensions::new(2, 1).unwrap(),
        rgb: vec![[0.8, 0.3, 0.2], [0.2, 0.5, 0.7]],
        alpha: vec![255, 128],
    };
    let before: Vec<[f32; 3]> = image
        .rgb
        .iter()
        .map(|rgb| linear_to_oklab(to_linear(*rgb)))
        .collect();
    let turn = decode_effects(&json!([{ "effect": "hue-saturation", "enabled": true, "hue": 90, "saturation": 0, "lightness": 0 }]).to_string()).unwrap().remove(0);
    turn.effect.apply(&mut image, &EffectContext::default());
    for (old, rgb) in before.iter().zip(&image.rgb) {
        let new = linear_to_oklab(to_linear(*rgb));
        assert!(
            (new[0] - old[0]).abs() < 1e-4,
            "lightness {old:?} -> {new:?}"
        );
        let chroma = |lab: &[f32; 3]| lab[1].hypot(lab[2]);
        assert!((chroma(&new) - chroma(old)).abs() < 1e-4);
        let turned = (new[2].atan2(new[1]) - old[2].atan2(old[1]))
            .to_degrees()
            .rem_euclid(360.0);
        assert!((turned - 90.0).abs() < 0.01, "turned {turned}");
    }
    let data = ramp();
    let grey = run(&data, json!([{ "effect": "hue-saturation", "enabled": true, "hue": 0, "saturation": -1, "lightness": 0 }])).unwrap();
    for (pixel, source) in grey.data().chunks(4).zip(data.chunks(4)) {
        let (low, high) = (
            pixel[..3].iter().min().unwrap(),
            pixel[..3].iter().max().unwrap(),
        );
        assert!(high - low <= 1, "{pixel:?}");
        assert_eq!(pixel[3], source[3]);
    }
    let white = run(&data, json!([{ "effect": "hue-saturation", "enabled": true, "hue": 0, "saturation": 0, "lightness": 1 }])).unwrap();
    let black = run(&data, json!([{ "effect": "hue-saturation", "enabled": true, "hue": 0, "saturation": 0, "lightness": -1 }])).unwrap();
    assert!(white.data().chunks(4).all(|p| p[..3] == [255, 255, 255]));
    assert!(black.data().chunks(4).all(|p| p[..3] == [0, 0, 0]));
}

#[test]
fn invalid_grading_arguments_name_their_field() {
    let data = ramp();
    let path = |effect: Value| run(&data, json!([effect])).unwrap_err().path;
    let curve = |points: Value| json!({ "effect": "curves", "enabled": true, "channel": "red", "points": points });
    assert_eq!(path(curve(json!([[0, 0]]))), "effects.0.points");
    assert_eq!(path(curve(json!(vec![[0, 0]; 17]))), "effects.0.points");
    assert_eq!(
        path(curve(json!([[0, 0], [0.5, 0.5], [0.5, 1]]))),
        "effects.0.points.2.0"
    );
    assert_eq!(
        path(curve(json!([[0, 0], [1, 1.5]]))),
        "effects.0.points.1.1"
    );
    assert_eq!(
        path(
            json!({ "effect": "brightness-contrast", "enabled": false, "brightness": 2, "contrast": 0 })
        ),
        "effects.0.brightness"
    );
    assert_eq!(
        path(json!({ "effect": "exposure", "enabled": true, "stops": 5 })),
        "effects.0.stops"
    );
    assert_eq!(
        path(json!({ "effect": "white-balance", "enabled": true, "temperature": 0, "tint": -1.5 })),
        "effects.0.tint"
    );
    assert_eq!(
        path(
            json!({ "effect": "hue-saturation", "enabled": true, "hue": 181, "saturation": 0, "lightness": 0 })
        ),
        "effects.0.hue"
    );
}

#[test]
fn close_knots_are_rejected_and_long_chains_stay_finite() {
    let data = ramp();
    let curve = json!({ "effect": "curves", "enabled": true, "channel": "red",
        "points": [[0, 0], [1e-25, 0.5], [1, 1]] });
    assert_eq!(
        run(&data, json!([curve])).unwrap_err().path,
        "effects.0.points.1.0"
    );

    // 33 boosts overflowed to infinity before the carrier bound; greys must stay light, not black.
    let mut chain: Vec<Value> =
        vec![json!({ "effect": "exposure", "enabled": true, "stops": 4 }); 33];
    chain.push(json!({ "effect": "hue-saturation", "enabled": true, "hue": 10, "saturation": 0, "lightness": 0 }));
    let grey = vec![128u8, 128, 128, 255].repeat(16);
    let output = run(&grey, Value::Array(chain)).unwrap();
    assert!(
        output
            .data()
            .chunks(4)
            .all(|pixel| pixel[..3].iter().all(|&c| c == 255)),
        "{:?}",
        &output.data()[..4]
    );
}
