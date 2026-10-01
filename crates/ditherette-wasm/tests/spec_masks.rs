//! Reference behaviour of step masks: strength per pixel from up to four curves.

use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions},
    spec::{
        contract::request::WorkingSpace,
        effects::{apply_chain, decode_effects, model::ColourModel, EffectContext, EffectImage},
    },
};
use serde_json::{json, Value};

const PALETTE: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [20, 40, 60] },
    PaletteEntry::Color {
        rgb: [230, 200, 90],
    },
];

const CONTEXT: EffectContext<'static> = EffectContext {
    palette: &PALETTE,
    space: Some(WorkingSpace::Oklab),
};

/// Greys, saturated colours, and overshooting carrier values.
const PIXELS: [[f32; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [0.5, 0.5, 0.5],
    [1.0, 1.0, 1.0],
    [0.9, 0.2, 0.1],
    [0.1, 0.7, 0.3],
    [0.2, 0.3, 0.95],
    [0.52, 0.5, 0.49],
    [1.2, -0.1, 0.4],
];

fn channel(model: &str, channel: &str) -> Value {
    json!({ "model": model, "channel": channel })
}

/// A one-input mask curve.
fn by(model: &str, name: &str, points: Value) -> Value {
    json!({ "x": channel(model, name), "points": points })
}

/// A mask curve that is `value` everywhere.
fn constant(value: f32) -> Value {
    by("oklch", "lightness", json!([[0, value], [1, value]]))
}

fn with_mask(mut step: Value, mask: Vec<Value>) -> Value {
    step["mask"] = Value::Array(mask);
    step
}

fn run(steps: &[Value], pixels: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let steps = decode_effects(&Value::from(steps.to_vec()).to_string()).unwrap();
    let mut image = EffectImage {
        dimensions: ImageDimensions::new(pixels.len() as u32, 1).unwrap(),
        rgb: pixels.to_vec(),
        alpha: vec![255; pixels.len()],
    };
    apply_chain(&mut image, &steps, &CONTEXT).unwrap();
    image.rgb
}

fn bits(pixels: &[[f32; 3]]) -> Vec<[u32; 3]> {
    pixels.iter().map(|rgb| rgb.map(f32::to_bits)).collect()
}

fn exposure(stops: f32) -> Value {
    json!({ "effect": "exposure", "enabled": true, "stops": stops })
}

fn lightness_remap(points: Value) -> Value {
    json!({ "effect": "curves", "enabled": true, "curves": [{
        "kind": "remap",
        "x": channel("oklab", "lightness"),
        "y": channel("oklab", "lightness"),
        "points": points,
    }] })
}

fn chroma_adjustment(value: f32) -> Value {
    json!({ "effect": "curves", "enabled": true, "curves": [{
        "kind": "adjust",
        "x": channel("srgb", "red"),
        "y": channel("oklch", "chroma"),
        "points": [[0, value], [1, value]],
    }] })
}

fn recolour(strength: f32) -> Value {
    json!({ "effect": "recolour", "enabled": true, "strength": strength, "recipe": {
        "space": "oklab", "tone": [[0, 0.1], [1, 0.9]], "chroma": 0.5, "shift": [0.02, -0.03],
        "groups": [{ "hue": 40, "width": 90, "turn": 30, "chroma": 1.5 }],
    } })
}

#[test]
fn empty_and_all_one_masks_equal_the_unmasked_step() {
    for step in [
        exposure(1.3),
        lightness_remap(json!([[0, 0.2], [0.6, 0.4], [1, 1]])),
        chroma_adjustment(0.8),
        recolour(0.7),
        json!({ "effect": "recolour", "enabled": true, "strength": 0.6, "recipe": null }),
    ] {
        let expected = bits(&run(&[step.clone()], &PIXELS));
        assert_eq!(
            bits(&run(&[with_mask(step.clone(), vec![])], &PIXELS)),
            expected
        );
        let ones = vec![
            constant(1.0),
            by("oklch", "hue", json!([[0, 1], [0.3, 1], [1, 1]])),
        ];
        assert_eq!(
            bits(&run(&[with_mask(step.clone(), ones)], &PIXELS)),
            expected,
            "{step}"
        );
    }
}

#[test]
fn empty_masks_leave_the_json_and_zero_masks_leave_the_input() {
    let decoded = decode_effects(&json!([with_mask(exposure(1.0), vec![])]).to_string()).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        json!([exposure(1.0)])
    );
    for step in [
        exposure(-2.0),
        lightness_remap(json!([[0, 1], [1, 0]])),
        chroma_adjustment(0.0),
        recolour(1.0),
    ] {
        let masked = with_mask(step, vec![constant(0.3), constant(0.0)]);
        assert_eq!(bits(&run(&[masked], &PIXELS)), bits(&PIXELS));
    }
}

#[test]
fn other_effects_blend_output_toward_input_by_the_product_of_curves() {
    let unmasked = run(&[exposure(1.5)], &PIXELS);
    let masked = run(
        &[with_mask(
            exposure(1.5),
            vec![constant(0.5), constant(0.25)],
        )],
        &PIXELS,
    );
    for ((input, output), actual) in PIXELS.iter().zip(&unmasked).zip(&masked) {
        let expected: [f32; 3] = std::array::from_fn(|c| input[c] + 0.125 * (output[c] - input[c]));
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
    }
}

#[test]
fn masks_read_the_pixel_entering_their_step() {
    let invert = json!({ "effect": "levels", "enabled": true, "channel": "rgb",
        "input": { "black": 0, "white": 1 }, "gamma": 1, "output": { "black": 1, "white": 0 } });
    let brighten = json!({ "effect": "brightness-contrast", "enabled": true,
        "brightness": 0.25, "contrast": 0 });
    // Full strength on light pixels only. After inversion, black is the light one.
    let lights = by("srgb", "red", json!([[0, 0], [0.5, 0], [1, 1]]));
    let out = run(
        &[invert, with_mask(brighten, vec![lights])],
        &[[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
    );
    assert_eq!(out, [[1.25; 3], [0.0; 3]]);
}

#[test]
fn hue_keyed_curves_fade_toward_one_on_near_greys() {
    let reds_off = by("oklch", "hue", json!([[0, 0], [1, 0]]));
    let masked = run(&[with_mask(exposure(1.0), vec![reds_off])], &PIXELS);
    let unmasked = run(&[exposure(1.0)], &PIXELS);
    // Exact greys have no hue, so the hue curve leaves them at full strength.
    for index in 0..3 {
        assert_eq!(
            masked[index].map(f32::to_bits),
            unmasked[index].map(f32::to_bits)
        );
    }
    // Saturated colours read the curve's 0 and stay put.
    for index in 3..6 {
        assert_eq!(masked[index], PIXELS[index]);
    }
    // A near-grey sits between: its strength is its hue confidence.
    let input = PIXELS[6];
    let coordinates = ColourModel::Oklch.to_normalized(input);
    let weight = ColourModel::Oklch.hue_weight(input, coordinates);
    assert!(weight > 0.0 && weight < 1.0, "{weight}");
    let strength = 1.0 - weight * (1.0 - 0.0);
    let expected: [f32; 3] =
        std::array::from_fn(|c| input[c] + strength * (unmasked[6][c] - input[c]));
    assert_eq!(masked[6].map(f32::to_bits), expected.map(f32::to_bits));
}

#[test]
fn two_input_curves_read_both_channels_and_take_the_lower_hue_confidence() {
    let grid = json!({
        "x": channel("oklch", "hue"),
        "x2": channel("oklch", "lightness"),
        "grid": { "columns": [0, 0.5], "rows": [0, 1], "values": [[0.4, 0.4], [0.4, 0.4]] },
    });
    let masked = run(&[with_mask(exposure(1.0), vec![grid])], &PIXELS);
    let flat = run(
        &[with_mask(exposure(1.0), vec![constant(0.4)])],
        &PIXELS[3..6],
    );
    assert_eq!(bits(&masked[3..6]), bits(&flat));
    assert_eq!(
        bits(&masked[..3]),
        bits(&run(&[exposure(1.0)], &PIXELS[..3]))
    );
}

#[test]
fn curves_scale_their_bend_from_neutral() {
    let source = [0.3, 0.45, 0.6];
    let half = constant(0.5);
    // Remap: x + m·(f(x) − x) in Oklab lightness, read from the source.
    let remap = lightness_remap(json!([[0, 1], [1, 1]]));
    let out = run(&[with_mask(remap, vec![half.clone()])], &[source])[0];
    let [lightness, a, b] = ColourModel::Oklab.to_normalized(source);
    let expected = ColourModel::Oklab.from_normalized([lightness + 0.5 * (1.0 - lightness), a, b]);
    assert_eq!(out.map(f32::to_bits), expected.map(f32::to_bits));
    // Adjustment: 0.5 + m·(c − 0.5), then the chroma gain.
    let out = run(&[with_mask(chroma_adjustment(1.0), vec![half])], &[source])[0];
    let [l, c, h] = ColourModel::Oklch.to_normalized(source);
    let curve: f32 = 0.5 + 0.5 * (1.0 - 0.5);
    let expected = ColourModel::Oklch.from_normalized([l, c * (1.0 + (2.0 * curve - 1.0)), h]);
    assert_eq!(out.map(f32::to_bits), expected.map(f32::to_bits));
}

#[test]
fn hue_remaps_scale_the_shortest_turn() {
    let source = [0.9, 0.2, 0.1];
    let hue = ColourModel::Hsl.to_normalized(source)[0];
    // A knot at the source hue sends it 0.1 back across the seam.
    let target = (hue - 0.1_f32).rem_euclid(1.0);
    let remap = json!({ "effect": "curves", "enabled": true, "curves": [{
        "kind": "remap", "x": channel("hsl", "hue"), "y": channel("hsl", "hue"),
        "points": [[0, 0.9], [hue, target], [1, 0.9]],
    }] });
    let out = run(&[with_mask(remap, vec![constant(0.5)])], &[source])[0];
    let after = ColourModel::Hsl.to_normalized(out)[0];
    let turned = (after - hue + 0.5).rem_euclid(1.0) - 0.5;
    assert!((turned + 0.05).abs() < 1e-4, "turned {turned}");
}

#[test]
fn recolour_multiplies_its_strength_by_the_mask() {
    let masked = run(&[with_mask(recolour(0.8), vec![constant(0.5)])], &PIXELS);
    assert_eq!(bits(&masked), bits(&run(&[recolour(0.4)], &PIXELS)));
    let automatic = |strength: f32| json!({ "effect": "recolour", "enabled": true, "strength": strength, "recipe": null });
    assert_eq!(
        bits(&run(
            &[with_mask(automatic(0.8), vec![constant(0.5)])],
            &PIXELS
        )),
        bits(&run(&[automatic(0.4)], &PIXELS))
    );
}

#[test]
fn masks_validate_limits_and_curves_with_paths() {
    let error = |mask: Vec<Value>, enabled: bool| {
        let mut step = with_mask(exposure(1.0), mask);
        step["enabled"] = json!(enabled);
        let steps = decode_effects(&json!([exposure(0.0), step]).to_string()).unwrap();
        let mut image = EffectImage {
            dimensions: ImageDimensions::new(1, 1).unwrap(),
            rgb: vec![[0.5; 3]],
            alpha: vec![255],
        };
        apply_chain(&mut image, &steps, &CONTEXT).unwrap_err().path
    };
    for enabled in [true, false] {
        assert_eq!(error(vec![constant(1.0); 5], enabled), "effects.1.mask");
        assert_eq!(
            error(
                vec![constant(1.0), by("oklch", "value", json!([[0, 1], [1, 1]]))],
                enabled
            ),
            "effects.1.mask.1.x.channel"
        );
        assert_eq!(
            error(
                vec![by("oklab", "a", json!([[0, 1], [0.0005, 1]]))],
                enabled
            ),
            "effects.1.mask.0.points.1.0"
        );
        assert_eq!(
            error(vec![by("oklab", "a", json!([[0, 1], [1, 1.5]]))], enabled),
            "effects.1.mask.0.points.1.1"
        );
        assert_eq!(
            error(vec![by("hsv", "hue", json!([[0, 1], [1, 0.5]]))], enabled),
            "effects.1.mask.0.points.1.1"
        );
        let same = json!({ "x": channel("oklch", "hue"), "x2": channel("oklch", "hue"),
            "grid": { "columns": [0, 0.5], "rows": [0, 0.5], "values": [[1, 1], [1, 1]] } });
        assert_eq!(error(vec![same], enabled), "effects.1.mask.0.x2");
        let value = json!({ "x": channel("oklch", "hue"), "x2": channel("oklab", "b"),
            "grid": { "columns": [0, 0.5], "rows": [0, 1], "values": [[1, 1], [1, -0.1]] } });
        assert_eq!(
            error(vec![value], enabled),
            "effects.1.mask.0.grid.values.1.1"
        );
    }
    // The step's own arguments fail before its mask.
    let bad_both = with_mask(exposure(9.0), vec![constant(1.0); 5]);
    let steps = decode_effects(&json!([bad_both]).to_string()).unwrap();
    let mut image = EffectImage {
        dimensions: ImageDimensions::new(1, 1).unwrap(),
        rgb: vec![[0.5; 3]],
        alpha: vec![255],
    };
    assert_eq!(
        apply_chain(&mut image, &steps, &CONTEXT).unwrap_err().path,
        "effects.0.stops"
    );
    // Mask curves are strict: no curve kind, no output channel, no mixed shapes.
    for curve in [
        json!({ "kind": "adjust", "x": channel("srgb", "red"), "points": [[0, 1], [1, 1]] }),
        json!({ "x": channel("srgb", "red"), "y": channel("srgb", "red"), "points": [[0, 1], [1, 1]] }),
        json!({ "x": channel("srgb", "red") }),
    ] {
        let json = json!([with_mask(exposure(1.0), vec![curve])]).to_string();
        assert_eq!(decode_effects(&json).unwrap_err().path, "effects.0");
    }
}
