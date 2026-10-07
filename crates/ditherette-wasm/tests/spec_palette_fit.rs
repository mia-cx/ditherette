//! Palette fit: analysis writes editable curves; the step applies them exactly like `curves`.

use ditherette_wasm::{
    image::contracts::{PaletteEntry, Rgba8Image},
    spec::{
        contract::{
            error::DitheretteError,
            request::{Source, WorkingSpace},
        },
        effects::{
            analyze_palette_fit, apply_effects,
            curves::{Curve, Curves},
            decode_effects,
            palette_fit::{FitLook, FitSpace},
            AnalyzePaletteFitRequest, Effect, EffectContext, EffectStep, EffectsRequest,
        },
    },
};
use serde_json::{json, Value};

const SIZE: u32 = 48;

/// A hue sweep across x and a lightness ramp down y, like a colourful photo in miniature.
fn sweep(saturation: f32) -> Vec<u8> {
    let mut data = Vec::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let hue = x as f32 / SIZE as f32 * 6.0;
            let value = 0.15 + 0.8 * y as f32 / (SIZE - 1) as f32;
            let f = hue.fract();
            let [p, q, t] = [
                1.0 - saturation,
                1.0 - saturation * f,
                1.0 - saturation * (1.0 - f),
            ];
            let rgb = match hue as u32 {
                0 => [1.0, t, p],
                1 => [q, 1.0, p],
                2 => [p, 1.0, t],
                3 => [p, q, 1.0],
                4 => [t, p, 1.0],
                _ => [1.0, p, q],
            };
            data.extend(rgb.map(|channel: f32| (channel * value * 255.0).round() as u8));
            data.push(255);
        }
    }
    data
}

fn colors(rgb: &[[u8; 3]]) -> Vec<PaletteEntry> {
    rgb.iter().map(|&rgb| PaletteEntry::Color { rgb }).collect()
}

fn wplace_free() -> Vec<PaletteEntry> {
    colors(&[
        [0, 0, 0],
        [60, 60, 60],
        [120, 120, 120],
        [210, 210, 210],
        [255, 255, 255],
        [96, 0, 24],
        [237, 28, 36],
        [255, 127, 39],
        [246, 170, 9],
        [249, 221, 59],
        [255, 250, 188],
        [14, 185, 104],
        [19, 230, 123],
        [135, 255, 94],
        [12, 129, 110],
        [16, 174, 130],
        [19, 225, 190],
        [96, 247, 242],
        [40, 80, 158],
        [64, 147, 228],
        [107, 80, 246],
        [153, 177, 251],
        [120, 12, 153],
        [170, 56, 185],
        [224, 159, 249],
        [203, 0, 122],
        [236, 31, 128],
        [243, 141, 169],
        [104, 70, 52],
        [149, 104, 42],
        [248, 178, 119],
    ])
}

fn warm() -> Vec<PaletteEntry> {
    colors(&[[96, 0, 24], [237, 28, 36], [255, 127, 39], [249, 221, 59]])
}

fn greys() -> Vec<PaletteEntry> {
    colors(&[[0, 0, 0], [85, 85, 85], [170, 170, 170], [255, 255, 255]])
}

fn source(data: &[u8]) -> Source<'_> {
    Source {
        width: SIZE,
        height: SIZE,
        data,
    }
}

fn steps(value: Value) -> Vec<EffectStep> {
    decode_effects(&value.to_string()).unwrap()
}

const LOOKS: [(&str, FitLook); 3] = [
    ("natural", FitLook::Natural),
    ("fitted", FitLook::Fitted),
    ("vivid", FitLook::Vivid),
];

fn fit_step(look: &str, space: &str, strength: f32, curves: Option<Value>) -> Value {
    json!({ "effect": "palette-fit", "enabled": true, "look": look,
        "space": space, "strength": strength, "curves": curves })
}

fn fit_in(space: &str, strength: f32, curves: Option<Value>) -> Value {
    fit_step("fitted", space, strength, curves)
}

fn fit(strength: f32, curves: Option<Value>) -> Value {
    fit_in("oklab", strength, curves)
}

fn analyze(
    data: &[u8],
    palette: &[PaletteEntry],
    space: FitSpace,
    look: FitLook,
    effects: &[EffectStep],
) -> Vec<Curve> {
    analyze_palette_fit(AnalyzePaletteFitRequest {
        version: 1,
        source: source(data),
        effects,
        context: EffectContext {
            palette,
            space: None,
        },
        space,
        look,
    })
    .unwrap()
}

fn run(
    data: &[u8],
    effects: &[EffectStep],
    palette: &[PaletteEntry],
    space: Option<WorkingSpace>,
) -> Result<Rgba8Image, DitheretteError> {
    apply_effects(EffectsRequest {
        version: 1,
        source: source(data),
        effects,
        context: EffectContext { palette, space },
    })
}

/// The analysed list, as steps: `palette-fit` auto, and the same list as a `curves` step.
fn curves_step(curves: &[Curve]) -> Value {
    json!({ "effect": "curves", "enabled": true, "curves": curves })
}

#[test]
fn analysis_writes_valid_curves_that_change_with_image_palette_and_space() {
    let (vivid, muted) = (sweep(0.9), sweep(0.3));
    let curves = analyze(
        &vivid,
        &wplace_free(),
        FitSpace::Oklab,
        FitLook::Fitted,
        &[],
    );
    assert!(!curves.is_empty() && curves.len() <= 5);
    Curves {
        curves: curves.clone(),
    }
    .validate("effects.0")
    .unwrap();
    assert_eq!(
        curves,
        analyze(
            &vivid,
            &wplace_free(),
            FitSpace::Oklab,
            FitLook::Fitted,
            &[]
        ),
        "deterministic"
    );
    assert_ne!(
        curves,
        analyze(
            &muted,
            &wplace_free(),
            FitSpace::Oklab,
            FitLook::Fitted,
            &[]
        ),
        "image content"
    );
    assert_ne!(
        curves,
        analyze(&vivid, &warm(), FitSpace::Oklab, FitLook::Fitted, &[]),
        "palette"
    );
    assert_ne!(
        curves,
        analyze(
            &vivid,
            &wplace_free(),
            FitSpace::Cielab,
            FitLook::Fitted,
            &[]
        ),
        "space"
    );
    // Analysis sees the image after the earlier steps.
    let levels = steps(
        json!([{ "effect": "levels", "enabled": true, "channel": "rgb",
        "input": { "black": 0.2, "white": 0.9 }, "gamma": 1.4, "output": { "black": 0, "white": 1 } }]),
    );
    assert_ne!(
        curves,
        analyze(
            &vivid,
            &wplace_free(),
            FitSpace::Oklab,
            FitLook::Fitted,
            &levels
        ),
        "earlier steps"
    );
}

#[test]
fn nothing_to_fit_means_no_curves() {
    let transparent: Vec<u8> = (0..SIZE * SIZE).flat_map(|_| [9, 9, 9, 0]).collect();
    assert!(analyze(&transparent, &warm(), FitSpace::Oklab, FitLook::Fitted, &[]).is_empty());
    // The step is then an exact no-op.
    let data = transparent;
    let output = run(&data, &steps(json!([fit(1.0, None)])), &warm(), None).unwrap();
    assert_eq!(output.data(), &data[..]);
    // A palette without visible colours cannot be analysed against.
    let only_transparent = [PaletteEntry::Transparent {}];
    assert_eq!(
        run(&sweep(0.9), &steps(json!([fit(1.0, None)])), &[], None)
            .unwrap_err()
            .path,
        "context.palette"
    );
    assert!(analyze_palette_fit(AnalyzePaletteFitRequest {
        version: 1,
        source: source(&sweep(0.9)),
        effects: &[],
        context: EffectContext {
            palette: &only_transparent,
            space: None,
        },
        space: FitSpace::Oklab,
        look: FitLook::Fitted,
    })
    .is_err());
}

#[test]
fn a_grey_image_gets_no_gain_or_turn() {
    let ramp: Vec<u8> = (0..SIZE * SIZE)
        .flat_map(|i| {
            let value = (i % 256) as u8;
            [value, value, value, 255]
        })
        .collect();
    for space in [FitSpace::Oklab, FitSpace::Cielab] {
        for (tag, look) in LOOKS {
            let curves = analyze(&ramp, &wplace_free(), space, look, &[]);
            for curve in &curves {
                let json = serde_json::to_value(curve).unwrap();
                let y = json["y"]["channel"].as_str().unwrap();
                assert_ne!(y, "chroma", "{tag}: gain curve on a grey image");
                assert_ne!(y, "hue", "{tag}: turn curve on a grey image");
            }
        }
    }
}

#[test]
fn application_matches_curves_with_the_same_list() {
    let data = sweep(0.8);
    let palette = wplace_free();
    for space in [FitSpace::Oklab, FitSpace::Cielab] {
        for (look_tag, look) in LOOKS {
            let curves = analyze(&data, &palette, space, look, &[]);
            let tag = match space {
                FitSpace::Oklab => "oklab",
                FitSpace::Cielab => "cielab",
            };
            let fitted = run(
                &data,
                &steps(json!([fit_step(look_tag, tag, 1.0, None)])),
                &palette,
                None,
            )
            .unwrap();
            let curved = run(&data, &steps(json!([curves_step(&curves)])), &palette, None).unwrap();
            assert_eq!(
                fitted, curved,
                "{space:?} {look_tag}: strength 1 without mask"
            );
            // The same list handed back explicit equals the automatic step.
            let explicit = run(
                &data,
                &steps(json!([fit_step(
                    look_tag,
                    tag,
                    1.0,
                    Some(serde_json::to_value(&curves).unwrap())
                )])),
                &palette,
                None,
            )
            .unwrap();
            assert_eq!(explicit, curved, "{space:?} {look_tag}: explicit curves");
        }
    }
}

#[test]
fn a_grey_palette_pulls_all_chroma_out() {
    // Zero reach in every hue: every cell's gain clamps to 0, so colours go grey.
    let data = sweep(0.9);
    let curves = analyze(&data, &greys(), FitSpace::Oklab, FitLook::Fitted, &[]);
    let gain = curves.iter().find_map(|curve| match curve {
        Curve::TwoInput(curve) => Some(curve),
        _ => None,
    });
    assert!(
        gain.is_some_and(|curve| curve
            .grid
            .values
            .iter()
            .flatten()
            .all(|&value| value == 0.0)),
        "zero gain on every cell"
    );
    let output = run(&data, &steps(json!([fit(1.0, None)])), &greys(), None).unwrap();
    assert!(output.data().chunks(4).all(|p| {
        let (low, high) = (p[..3].iter().min().unwrap(), p[..3].iter().max().unwrap());
        high - low <= 1
    }));
}

#[test]
fn strength_scales_the_bend_and_mask_multiplies_it() {
    let data = sweep(0.8);
    let palette = wplace_free();
    assert_eq!(
        run(&data, &steps(json!([fit(0.0, None)])), &palette, None)
            .unwrap()
            .data(),
        &data[..],
        "strength 0 is an exact no-op"
    );
    assert_eq!(
        run(
            &data,
            &steps(json!([fit(1.0, Some(json!([])))])),
            &palette,
            None
        )
        .unwrap()
        .data(),
        &data[..],
        "an empty list is an exact no-op"
    );
    // strength * mask is the curves mask strength: a constant 0.5 mask at strength 1 equals
    // strength 0.5 unmasked, on every pixel.
    let curves = analyze(&data, &palette, FitSpace::Oklab, FitLook::Fitted, &[]);
    let masked = json!([{
        "effect": "palette-fit", "enabled": true, "look": "fitted", "space": "oklab",
        "strength": 1.0, "curves": null,
        "mask": [{ "x": { "model": "srgb", "channel": "red" }, "points": [[0, 0.5], [1, 0.5]] }]
    }]);
    let half = json!([fit(0.5, None)]);
    assert_eq!(
        run(&data, &steps(masked.clone()), &palette, None).unwrap(),
        run(&data, &steps(half), &palette, None).unwrap(),
        "m = strength * mask"
    );
    let half_curves = json!([{
        "effect": "curves", "enabled": true, "curves": curves,
        "mask": [{ "x": { "model": "srgb", "channel": "red" }, "points": [[0, 0.5], [1, 0.5]] }]
    }]);
    assert_eq!(
        run(&data, &steps(masked.clone()), &palette, None).unwrap(),
        run(&data, &steps(half_curves), &palette, None).unwrap(),
        "the curves effect at mask 0.5"
    );
}

#[test]
fn explicit_curves_ignore_the_context_but_their_fields_validate() {
    let data = sweep(0.5);
    let curves = analyze(&data, &wplace_free(), FitSpace::Oklab, FitLook::Fitted, &[]);
    let list = serde_json::to_value(&curves).unwrap();
    // No palette and no space needed once the list is explicit.
    let output = run(
        &data,
        &steps(json!([fit(0.8, Some(list.clone()))])),
        &[],
        None,
    )
    .unwrap();
    assert_eq!(
        output,
        run(
            &data,
            &steps(json!([fit(0.8, Some(list.clone()))])),
            &warm(),
            Some(WorkingSpace::Cielab)
        )
        .unwrap(),
        "context is irrelevant with explicit curves"
    );
    let path = |effect: Value, palette: &[PaletteEntry], space| {
        run(&data, &steps(json!([effect])), palette, space)
            .unwrap_err()
            .path
    };
    assert_eq!(
        path(fit(3.5, Some(list.clone())), &[], None),
        "effects.0.strength"
    );
    assert_eq!(path(fit(1.0, None), &[], None), "context.palette");
    // A palette fit analyses in its own space: no context space is required.
    run(&data, &steps(json!([fit(1.0, None)])), &warm(), None).unwrap();
    // Curves validate under the step's own path, like the curves effect's list.
    let mut bad = list.clone();
    bad.as_array_mut().unwrap().push(json!({
        "kind": "remap", "x": { "model": "oklab", "channel": "lightness" },
        "y": { "model": "oklab", "channel": "lightness" },
        "points": [[0, 0], [0.0005, 0.5]]
    }));
    assert_eq!(
        path(fit(1.0, Some(bad)), &[], None),
        format!("effects.0.curves.{}.points.1.0", curves.len()),
        "curve validation paths"
    );
    // An unknown look or space fails to decode at the step.
    let mut bad_look = fit(1.0, None);
    bad_look["look"] = json!("gentle");
    assert!(decode_effects(&json!([bad_look]).to_string()).is_err());
    let mut bad_space = fit(1.0, None);
    bad_space["space"] = json!("hsv");
    assert!(decode_effects(&json!([bad_space]).to_string()).is_err());
}

#[test]
fn a_preceding_recolour_reads_the_context_space() {
    // A recipe-less recolour before the fit still analyses: the context space belongs to it,
    // not to the fit, which keeps its own space.
    let data = sweep(0.9);
    let preceding =
        steps(json!([{ "effect": "recolour", "enabled": true, "strength": 0.8, "recipe": null }]));
    let palette = wplace_free();
    let analyze = |space: Option<WorkingSpace>| {
        analyze_palette_fit(AnalyzePaletteFitRequest {
            version: 1,
            source: source(&data),
            effects: &preceding,
            context: EffectContext {
                palette: &palette,
                space,
            },
            space: FitSpace::Oklab,
            look: FitLook::Fitted,
        })
    };
    assert_eq!(analyze(None).unwrap_err().path, "context.space");
    let curves = analyze(Some(WorkingSpace::Oklab)).unwrap();
    assert!(!curves.is_empty());
}

#[test]
fn natural_emits_no_shift_or_turn_and_never_grows_chroma() {
    let data = sweep(0.9);
    for space in [FitSpace::Oklab, FitSpace::Cielab] {
        let curves = analyze(&data, &warm(), space, FitLook::Natural, &[]);
        for curve in &curves {
            let json = serde_json::to_value(curve).unwrap();
            let y = json["y"]["channel"].as_str().unwrap();
            assert!(
                y != "a" && y != "b",
                "{space:?}: natural emits no shift curve"
            );
            assert!(
                !(json["x"]["channel"].as_str() == Some("hue") && y == "hue"),
                "{space:?}: natural emits no turn curve"
            );
        }
        if let Some(gain) = curves.iter().find_map(|curve| match curve {
            Curve::TwoInput(curve) => Some(curve),
            _ => None,
        }) {
            assert!(
                gain.grid.values.iter().flatten().all(|&value| value <= 0.5),
                "{space:?}: natural gains never grow chroma"
            );
        }
    }
}

#[test]
fn vivid_reaches_more_chroma_than_fitted_on_a_muted_image() {
    // A muted image against a saturated palette has cells fitted clamps at 1.25; vivid may
    // reach the slice's full reach (2).
    let data = sweep(0.25);
    let saturated = colors(&[
        [0, 0, 0],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
        [255, 255, 0],
        [0, 255, 255],
        [255, 0, 255],
        [255, 255, 255],
    ]);
    let max_gain = |look: FitLook| {
        analyze(&data, &saturated, FitSpace::Oklab, look, &[])
            .iter()
            .find_map(|curve| match curve {
                Curve::TwoInput(curve) => Some(
                    curve
                        .grid
                        .values
                        .iter()
                        .flatten()
                        .fold(0.0f32, |a, &b| a.max(b)),
                ),
                _ => None,
            })
            .unwrap_or(0.0)
    };
    let fitted = max_gain(FitLook::Fitted);
    assert!(fitted > 0.5, "fitted should grow chroma here: {fitted}");
    assert!(max_gain(FitLook::Vivid) > fitted, "vivid exceeds fitted");
    assert!(
        max_gain(FitLook::Natural) <= 0.5,
        "natural stays at or below neutral"
    );
}

#[test]
fn strength_past_one_scales_the_bend_and_clamps_channels() {
    // The list a fit applies is a curves list at mask strength `m = strength`, so m up to 3
    // is legal here and only here: a chroma gain of 0 at 300% floors chroma and greys the
    // pixel, where an unclamped negative gain would bounce to the opposite hue.
    let data = sweep(0.9);
    let muted_palette = wplace_free();
    let kill_chroma = json!([
        { "kind": "adjust", "x": { "model": "oklch", "channel": "lightness" },
          "y": { "model": "oklch", "channel": "chroma" },
          "points": [[0, 0], [1, 0]] }
    ]);
    for out in [
        run(
            &data,
            &steps(json!([fit_in("oklab", 3.0, Some(kill_chroma.clone()))])),
            &muted_palette,
            None,
        )
        .unwrap(),
        run(
            &data,
            &steps(json!([fit_step(
                "vivid",
                "cielab",
                3.0,
                Some(kill_chroma.clone())
            )])),
            &muted_palette,
            None,
        )
        .unwrap(),
    ] {
        for pixel in out.data().chunks_exact(4) {
            assert_eq!(
                pixel[0], pixel[1],
                "grey after chroma floors at 300%: {pixel:?}"
            );
            assert_eq!(pixel[1], pixel[2]);
        }
    }

    // A remap scaled past 1 clamps the channel before converting back: oklab `a` at 300%
    // lands on the same bytes as `a` remapped to 1 at full strength, not on the bytes an
    // unclamped coordinate would give.
    let top_a = json!([
        { "kind": "remap", "x": { "model": "oklab", "channel": "a" },
          "y": { "model": "oklab", "channel": "a" },
          "points": [[0, 1], [1, 1]] }
    ]);
    let at_one = run(
        &data,
        &steps(json!([fit(1.0, Some(top_a.clone()))])),
        &muted_palette,
        None,
    )
    .unwrap();
    let at_three = run(
        &data,
        &steps(json!([fit(3.0, Some(top_a))])),
        &muted_palette,
        None,
    )
    .unwrap();
    assert_eq!(
        at_three.data(),
        at_one.data(),
        "the overshot remap clamps to the channel bound"
    );
    // And it is the bound, not the unclamped value: the opposite pull differs.
    let bottom_a = json!([
        { "kind": "remap", "x": { "model": "oklab", "channel": "a" },
          "y": { "model": "oklab", "channel": "a" },
          "points": [[0, 0], [1, 0]] }
    ]);
    assert_ne!(
        at_three.data(),
        run(
            &data,
            &steps(json!([fit(1.0, Some(bottom_a))])),
            &muted_palette,
            None,
        )
        .unwrap()
        .data()
    );

    // And strengths 2 and 3 validate; 3.01 does not.
    let mut over = fit(1.0, None);
    over["strength"] = json!(3.01);
    assert_eq!(
        steps(json!([over]))[0]
            .effect
            .validate("effects.0")
            .unwrap_err()
            .path,
        "effects.0.strength"
    );
}
