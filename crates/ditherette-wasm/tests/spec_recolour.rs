//! Recolouring: analysis responds to image, palette, and space; recipes are exact data.

use ditherette_wasm::{
    image::contracts::{PaletteEntry, Rgba8Image},
    spec::{
        contract::{
            error::DitheretteError,
            request::{Source, WorkingSpace},
        },
        effects::{
            analyze_recolour, apply_effects, decode_effects, process,
            recolour::{Group, RecolourRecipe},
            AnalyzeRequest, EffectContext, EffectStep, EffectsRequest, ProcessRequestV2,
        },
        pipeline,
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

fn analyze(
    data: &[u8],
    palette: &[PaletteEntry],
    space: WorkingSpace,
    effects: &[EffectStep],
) -> RecolourRecipe {
    analyze_recolour(AnalyzeRequest {
        version: 1,
        source: source(data),
        effects,
        context: EffectContext {
            palette,
            space: Some(space),
        },
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

fn recolour(strength: f32, recipe: Option<&RecolourRecipe>) -> Value {
    json!({ "effect": "recolour", "enabled": true, "strength": strength, "recipe": recipe })
}

#[test]
fn analysis_responds_to_image_palette_and_space() {
    let (vivid, muted) = (sweep(0.9), sweep(0.3));
    let oklab = WorkingSpace::Oklab;
    let full = analyze(&vivid, &wplace_free(), oklab, &[]);
    assert_ne!(
        full,
        analyze(&muted, &wplace_free(), oklab, &[]),
        "image content"
    );
    assert_ne!(
        full,
        analyze(&vivid, &warm(), oklab, &[]),
        "restricted palette"
    );
    assert_ne!(
        full,
        analyze(&vivid, &wplace_free()[..8], oklab, &[]),
        "palette subset"
    );
    let cielab = analyze(&vivid, &wplace_free(), WorkingSpace::Cielab, &[]);
    assert_eq!(cielab.space, WorkingSpace::Cielab);
    assert_ne!(cielab.tone, full.tone, "working space");
    assert_eq!(
        full,
        analyze(&vivid, &wplace_free(), oklab, &[]),
        "deterministic"
    );
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklch,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        let recipe = analyze(&vivid, &warm(), space, &[]);
        recipe.validate("recipe").unwrap();
        assert!(!recipe.is_identity(), "{space:?}");
    }
}

#[test]
fn palette_reach_shapes_the_treatment() {
    let vivid = sweep(0.9);
    let space = WorkingSpace::Oklab;
    // A grey palette cannot mix any hue, so colour goes before quantization.
    let grey = analyze(&vivid, &greys(), space, &[]);
    assert_eq!(grey.chroma, 0.0);
    let output = run(
        &vivid,
        &steps(json!([recolour(1.0, Some(&grey))])),
        &greys(),
        Some(space),
    )
    .unwrap();
    assert!(output.data().chunks(4).all(|p| {
        let (low, high) = (p[..3].iter().min().unwrap(), p[..3].iter().max().unwrap());
        high - low <= 1
    }));
    // Warm-only colours exclude neutral from the hull, and blues must turn or mute.
    let warm = analyze(&vivid, &warm(), space, &[]);
    assert_ne!(warm.shift, [0.0, 0.0]);
    assert!(warm
        .groups
        .iter()
        .any(|group| group.turn != 0.0 || group.chroma < 1.0));
    // A palette spanning neutral needs no shift.
    assert_eq!(
        analyze(&vivid, &wplace_free(), space, &[]).shift,
        [0.0, 0.0]
    );
}

#[test]
fn strength_and_identity_are_exact_and_recipes_reapply() {
    let data = sweep(0.8);
    let space = Some(WorkingSpace::Oklab);
    let palette = warm();
    for strength_zero in [
        recolour(0.0, None),
        recolour(
            0.0,
            Some(&analyze(&data, &palette, WorkingSpace::Oklab, &[])),
        ),
    ] {
        assert_eq!(
            run(&data, &steps(json!([strength_zero])), &palette, space)
                .unwrap()
                .data(),
            data
        );
    }
    let identity = RecolourRecipe::identity(WorkingSpace::Oklab);
    assert_eq!(
        run(
            &data,
            &steps(json!([recolour(1.0, Some(&identity))])),
            &palette,
            space
        )
        .unwrap()
        .data(),
        data
    );

    // Automatic analysis equals applying the recipe analyze_recolour returns, at any strength.
    let recipe = analyze(&data, &palette, WorkingSpace::Oklab, &[]);
    for strength in [0.35, 1.0] {
        let auto = run(
            &data,
            &steps(json!([recolour(strength, None)])),
            &palette,
            space,
        )
        .unwrap();
        let explicit = run(
            &data,
            &steps(json!([recolour(strength, Some(&recipe))])),
            &palette,
            space,
        )
        .unwrap();
        assert_eq!(auto, explicit, "strength {strength}");
    }
    // Recipes round-trip as JSON, and an edit changes the result deterministically.
    let json = serde_json::to_string(&recipe).unwrap();
    assert_eq!(
        serde_json::from_str::<RecolourRecipe>(&json).unwrap(),
        recipe
    );
    let mut edited = recipe.clone();
    edited.groups.push(Group {
        hue: 240.0,
        width: 60.0,
        turn: 30.0,
        chroma: 1.2,
    });
    let first = run(
        &data,
        &steps(json!([recolour(1.0, Some(&edited))])),
        &palette,
        space,
    )
    .unwrap();
    let second = run(
        &data,
        &steps(json!([recolour(1.0, Some(&edited))])),
        &palette,
        space,
    )
    .unwrap();
    assert_eq!(first, second);
    assert_ne!(
        first,
        run(
            &data,
            &steps(json!([recolour(1.0, Some(&recipe))])),
            &palette,
            space
        )
        .unwrap()
    );
}

#[test]
fn gradients_keep_intermediate_colours() {
    // A 256-step grey ramp: recolouring for a 4-grey palette must not posterize it.
    let ramp: Vec<u8> = (0..SIZE * SIZE)
        .flat_map(|i| {
            let value = (i % 256) as u8;
            [value, value, value, 255]
        })
        .collect();
    let output = run(
        &ramp,
        &steps(json!([recolour(1.0, None)])),
        &greys(),
        Some(WorkingSpace::Oklab),
    )
    .unwrap();
    let mut distinct: Vec<u8> = output.data().chunks(4).map(|p| p[0]).collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert!(
        distinct.len() > 200,
        "only {} levels survive",
        distinct.len()
    );
}

#[test]
fn recolouring_composes_with_earlier_effects_and_every_dither_family() {
    let data = sweep(0.7);
    let space = WorkingSpace::Oklab;
    let levels = json!({ "effect": "levels", "enabled": true, "channel": "rgb",
        "input": { "black": 0.2, "white": 0.9 }, "gamma": 1.4, "output": { "black": 0, "white": 1 } });
    let prefix = steps(json!([levels]));
    // Analysis sees the image after the earlier effects, not the source.
    let after = analyze(&data, &warm(), space, &prefix);
    assert_ne!(after, analyze(&data, &warm(), space, &[]));
    let auto = run(
        &data,
        &steps(json!([levels, recolour(1.0, None)])),
        &warm(),
        Some(space),
    )
    .unwrap();
    let explicit = run(
        &data,
        &steps(json!([levels, recolour(1.0, Some(&after))])),
        &warm(),
        Some(space),
    )
    .unwrap();
    assert_eq!(auto, explicit);

    for dither in [
        json!({ "family": "none" }),
        json!({ "family": "separable", "perturb": { "field": { "algorithm": "bayer", "size": "4" },
            "space": "oklab", "strength": 0.6, "placement": { "mode": "everywhere" } } }),
        json!({ "family": "diffusion", "kernel": "atkinson", "strength": 1.0,
            "placement": { "mode": "everywhere" }, "serpentine": false, "feedback": "matching" }),
        json!({ "family": "yliluoma", "size": "2", "placement": { "mode": "everywhere" } }),
    ] {
        let recipe = ditherette_wasm::spec::effects::decode_recipe_v2(
            &json!({ "version": 2, "effects": [levels, recolour(1.0, None)],
                "output": { "width": 24, "height": 24, "resize": { "algorithm": "area" } },
                "alpha": { "mode": "preserve", "threshold": 127.5 }, "match": "oklab-euclidean",
                "dither": dither })
            .to_string(),
        )
        .unwrap();
        let palette = warm();
        let composed = process(ProcessRequestV2 {
            source: source(&data),
            palette: &palette,
            recipe: &recipe,
        })
        .unwrap();
        let staged = pipeline::process(ditherette_wasm::spec::contract::request::ProcessRequest {
            source: source(auto.data()),
            palette: &palette,
            recipe: recipe.terminal(),
        })
        .unwrap();
        assert_eq!(composed, staged, "{dither}");
        assert!(composed
            .indices
            .data()
            .iter()
            .all(|&index| (index as usize) < palette.len()));
    }
}

#[test]
fn recolour_arguments_and_context_are_validated() {
    let data = sweep(0.5);
    let palette = warm();
    let space = Some(WorkingSpace::Oklab);
    let path = |effect: Value, palette: &[PaletteEntry], space| {
        run(&data, &steps(json!([effect])), palette, space)
            .unwrap_err()
            .path
    };
    assert_eq!(
        path(recolour(1.5, None), &palette, space),
        "effects.0.strength"
    );
    assert_eq!(path(recolour(1.0, None), &[], space), "context.palette");
    assert_eq!(path(recolour(1.0, None), &palette, None), "context.space");
    let recipe = analyze(&data, &palette, WorkingSpace::Oklab, &[]);
    // An explicit recipe needs no palette, but its space must match the context.
    assert!(run(
        &data,
        &steps(json!([recolour(1.0, Some(&recipe))])),
        &[],
        space
    )
    .is_ok());
    assert_eq!(
        path(
            recolour(1.0, Some(&recipe)),
            &[],
            Some(WorkingSpace::Cielab)
        ),
        "effects.0.recipe.space"
    );
    let mut bad = recipe.clone();
    bad.tone = vec![[0.0, 0.0], [0.0005, 1.0]];
    assert_eq!(
        path(recolour(1.0, Some(&bad)), &palette, space),
        "effects.0.recipe.tone.1.0"
    );
    let mut bad = recipe.clone();
    bad.groups = vec![
        Group {
            hue: 0.0,
            width: 60.0,
            turn: 0.0,
            chroma: 1.0
        };
        13
    ];
    assert_eq!(
        path(recolour(1.0, Some(&bad)), &palette, space),
        "effects.0.recipe.groups"
    );
    let mut bad = recipe;
    bad.shift = [0.0, 0.7];
    assert_eq!(
        path(recolour(1.0, Some(&bad)), &palette, space),
        "effects.0.recipe.shift.1"
    );
    // Transparent pixels cannot steer analysis.
    let clear: Vec<u8> = data.chunks(4).flat_map(|p| [p[0], p[1], p[2], 0]).collect();
    assert!(analyze(&clear, &palette, WorkingSpace::Oklab, &[]).is_identity());
}
