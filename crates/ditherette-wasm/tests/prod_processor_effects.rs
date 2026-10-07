//! The production processor's effect calls equal the frozen effects reference.

use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, PaletteIndex8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::{Progress, Stage},
            request::{RecipeV1, WorkingSpace},
        },
        effects as prod_effects,
        pipeline::{
            effects::EffectsRequest,
            process::ProcessRequest,
            processor::{Boundary, Processor},
            progress::Callback,
            quantize::{IndexedMetadataRef, InputBoundary, QuantizeBoundary},
        },
    },
    spec::{self, contract::request::Source},
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
const WIDTH: u32 = 16;
const HEIGHT: u32 = 16;

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

#[derive(Default)]
struct Events(Vec<Stage>);

impl Callback for Events {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        Ok(self.0.len() as u64 * 100)
    }

    fn report(&mut self, progress: Progress) -> Result<(), ()> {
        self.0.push(progress.stage);
        Ok(())
    }
}

struct Io<'a> {
    data: &'a [u8],
    events: Option<Events>,
}

impl InputBoundary for Io<'_> {
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        self.events
            .as_mut()
            .map(|events| events as &mut dyn Callback)
    }

    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.data.len())
    }

    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.data);
        Ok(())
    }

    /// Compares like the Wasm boundary, so warm calls exercise snapshot reuse.
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        if compare && destination == self.data {
            return Ok(true);
        }
        destination.copy_from_slice(self.data);
        Ok(false)
    }
}

impl Boundary for Io<'_> {
    type Output = Vec<u8>;

    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        Ok(bytes.to_vec())
    }
}

impl QuantizeBoundary for Io<'_> {
    type Output = IndexedImage;

    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<IndexedImage, Failure> {
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .unwrap(),
            palette: palette.palette.clone(),
            warnings: palette.warnings.to_vec(),
        })
    }
}

fn chain() -> serde_json::Value {
    json!([
        { "effect": "levels", "enabled": true, "channel": "rgb",
          "input": { "black": 0.1, "white": 0.8 }, "gamma": 1.3,
          "output": { "black": 0.0, "white": 1.0 } },
        { "effect": "levels", "enabled": false, "channel": "red",
          "input": { "black": 0.0, "white": 1.0 }, "gamma": 4.0,
          "output": { "black": 1.0, "white": 0.0 } },
        { "effect": "levels", "enabled": true, "channel": "blue",
          "input": { "black": 0.0, "white": 1.0 }, "gamma": 1.0,
          "output": { "black": 0.3, "white": 0.6 } },
    ])
}

fn processor() -> Processor {
    Processor::new(64 * 1024 * 1024, 0).unwrap()
}

fn spec_effects(data: &[u8], effects: &serde_json::Value) -> Vec<u8> {
    let steps = spec::effects::decode_effects(&effects.to_string()).unwrap();
    spec::effects::apply_effects(spec::effects::EffectsRequest {
        version: 1,
        source: Source {
            width: WIDTH,
            height: HEIGHT,
            data,
        },
        effects: &steps,
        context: spec::effects::EffectContext::default(),
    })
    .unwrap()
    .into_vec()
}

#[test]
fn apply_effects_matches_the_reference_cold_and_warm() {
    let data = ramp();
    let effects = prod_effects::decode_effects(&chain().to_string()).unwrap();
    let expected = spec_effects(&data, &chain());
    let mut processor = processor();
    for _ in 0..2 {
        let mut io = Io {
            data: &data,
            events: Some(Events::default()),
        };
        let output = processor
            .apply_effects(
                EffectsRequest {
                    source_width: WIDTH,
                    source_height: HEIGHT,
                    effects: &effects,
                    context: prod_effects::EffectContext::default(),
                },
                &mut io,
            )
            .unwrap();
        assert_eq!(output, expected);
        let events = io.events.unwrap().0;
        assert_eq!(events.first(), Some(&Stage::Prepare));
        assert_eq!(events.last(), Some(&Stage::Complete));
    }
}

fn recipe(dither: serde_json::Value) -> spec::effects::RecipeV2 {
    spec::effects::decode_recipe_v2(
        &json!({
            "version": 2,
            "effects": chain(),
            "output": { "width": 7, "height": 5, "resize": { "algorithm": "bilinear", "anchor": "center" } },
            "alpha": { "mode": "preserve", "threshold": 127.5 },
            "match": "oklab-euclidean",
            "dither": dither,
        })
        .to_string(),
    )
    .unwrap()
}

fn prod_recipe(recipe: &spec::effects::RecipeV2) -> RecipeV1 {
    let mut terminal = serde_json::to_value(recipe.terminal()).unwrap();
    terminal["version"] = json!(1);
    serde_json::from_value(terminal).unwrap()
}

#[test]
fn process_effects_matches_the_reference_for_every_dither_family() {
    let data = ramp();
    let dithers = [
        json!({ "family": "none" }),
        json!({ "family": "separable", "perturb": {
            "field": { "algorithm": "bayer", "size": "4" }, "space": "oklab",
            "strength": 0.6, "placement": { "mode": "everywhere" } } }),
        json!({ "family": "diffusion", "kernel": "sierra", "strength": 0.9,
            "placement": { "mode": "everywhere" }, "serpentine": false, "feedback": "srgb-bytes" }),
        json!({ "family": "yliluoma", "size": "2", "placement": { "mode": "everywhere" } }),
    ];
    let effects = prod_effects::decode_effects(&chain().to_string()).unwrap();
    let mut processor = processor();
    let mut cold = true;
    for dither in dithers {
        let recipe = recipe(dither.clone());
        let expected = spec::effects::process(spec::effects::ProcessRequestV2 {
            source: Source {
                width: WIDTH,
                height: HEIGHT,
                data: &data,
            },
            palette: &PALETTE,
            recipe: &recipe,
        })
        .unwrap();
        // The source and chain never change, so every call after the very first reuses the
        // retained source and skips the effects entirely, whatever the dither.
        for _ in 0..2 {
            let warm = !std::mem::replace(&mut cold, false);
            let mut io = Io {
                data: &data,
                events: Some(Events::default()),
            };
            let actual = processor
                .process_effects(
                    ProcessRequest {
                        source_width: WIDTH,
                        source_height: HEIGHT,
                        palette: &PALETTE,
                        recipe: prod_recipe(&recipe),
                    },
                    &effects,
                    &mut io,
                )
                .unwrap();
            assert_eq!(actual, expected, "{dither}");
            let events = io.events.unwrap().0;
            let effects_at = events.iter().position(|stage| *stage == Stage::Effects);
            assert_eq!(events.first(), Some(&Stage::Prepare), "{dither}");
            assert_eq!(events.last(), Some(&Stage::Complete), "{dither}");
            if warm {
                assert_eq!(effects_at, None, "{dither}: {events:?}");
            } else {
                assert!(
                    effects_at.is_some_and(|index| index > 0),
                    "{dither}: {events:?}"
                );
            }
        }
    }
}

#[test]
fn disabled_chains_still_use_v2_coverage_resize() {
    let data = ramp();
    let mut effects = prod_effects::decode_effects(&chain().to_string()).unwrap();
    for step in &mut effects {
        step.enabled = false;
    }
    let recipe = recipe(json!({ "family": "none" }));
    let request = ProcessRequest {
        source_width: WIDTH,
        source_height: HEIGHT,
        palette: &PALETTE,
        recipe: prod_recipe(&recipe),
    };
    let mut io = Io {
        data: &data,
        events: None,
    };
    let actual = processor()
        .process_effects(request, &effects, &mut io)
        .unwrap();
    let mut reference = recipe;
    for step in &mut reference.effects {
        step.enabled = false;
    }
    let expected = spec::effects::process(spec::effects::ProcessRequestV2 {
        source: Source {
            width: WIDTH,
            height: HEIGHT,
            data: &data,
        },
        palette: &PALETTE,
        recipe: &reference,
    })
    .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn invalid_effects_fail_before_any_work_and_leave_the_instance_usable() {
    let data = ramp();
    let mut bad = chain();
    bad[1]["gamma"] = json!(0.0);
    let effects = prod_effects::decode_effects(&bad.to_string()).unwrap();
    let mut processor = processor();
    let mut io = Io {
        data: &data,
        events: None,
    };
    let request = EffectsRequest {
        source_width: WIDTH,
        source_height: HEIGHT,
        effects: &effects,
        context: prod_effects::EffectContext::default(),
    };
    assert_eq!(
        processor.apply_effects(request, &mut io).unwrap_err(),
        Failure::new(ErrorCode::InvalidSettings, ErrorPath::Effects)
    );
    let good = prod_effects::decode_effects(&chain().to_string()).unwrap();
    let output = processor
        .apply_effects(
            EffectsRequest {
                effects: &good,
                ..request
            },
            &mut io,
        )
        .unwrap();
    assert_eq!(output, spec_effects(&data, &chain()));
}

fn recolour_chain(before: serde_json::Value, after: serde_json::Value) -> serde_json::Value {
    let mut chain = vec![before];
    chain.push(json!({ "effect": "recolour", "enabled": true, "strength": 0.8, "recipe": null }));
    chain.push(after);
    serde_json::Value::Array(chain)
}

fn levels_gamma(gamma: f32) -> serde_json::Value {
    json!({ "effect": "levels", "enabled": true, "channel": "rgb",
        "input": { "black": 0.0, "white": 1.0 }, "gamma": gamma,
        "output": { "black": 0.0, "white": 1.0 } })
}

#[test]
fn recolour_analyses_are_cached_by_what_they_read() {
    let data = ramp();
    let mut processor = processor();
    let space = WorkingSpace::Oklab;
    let run = |processor: &mut Processor, chain: &serde_json::Value| {
        let effects = prod_effects::decode_effects(&chain.to_string()).unwrap();
        let expected = spec::effects::apply_effects(spec::effects::EffectsRequest {
            version: 1,
            source: Source {
                width: WIDTH,
                height: HEIGHT,
                data: &data,
            },
            effects: &spec::effects::decode_effects(&chain.to_string()).unwrap(),
            context: spec::effects::EffectContext {
                palette: &PALETTE,
                space: Some(ditherette_wasm::spec::contract::request::WorkingSpace::Oklab),
            },
        })
        .unwrap()
        .into_vec();
        let mut io = Io {
            data: &data,
            events: None,
        };
        let actual = processor
            .apply_effects(
                EffectsRequest {
                    source_width: WIDTH,
                    source_height: HEIGHT,
                    effects: &effects,
                    context: prod_effects::EffectContext {
                        palette: &PALETTE,
                        space: Some(space),
                        analyses: None,
                    },
                },
                &mut io,
            )
            .unwrap();
        assert_eq!(actual, expected, "{chain}");
    };
    run(
        &mut processor,
        &recolour_chain(levels_gamma(1.2), levels_gamma(0.9)),
    );
    assert_eq!(processor.cached_analyses(), 1);
    // A different effect after the recolour step reuses its analysis.
    run(
        &mut processor,
        &recolour_chain(levels_gamma(1.2), levels_gamma(1.7)),
    );
    assert_eq!(processor.cached_analyses(), 1);
    // A different effect before it changes what analysis reads.
    run(
        &mut processor,
        &recolour_chain(levels_gamma(2.0), levels_gamma(1.7)),
    );
    assert_eq!(processor.cached_analyses(), 2);

    // Standalone analysis of the same prefix is a hit and returns the same recipe as the reference.
    let prefix = prod_effects::decode_effects(&json!([levels_gamma(2.0)]).to_string()).unwrap();
    let mut io = Io {
        data: &data,
        events: None,
    };
    let recipe = processor
        .analyze_recolour(
            EffectsRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                effects: &prefix,
                context: prod_effects::EffectContext {
                    palette: &PALETTE,
                    space: Some(space),
                    analyses: None,
                },
            },
            &mut io,
        )
        .unwrap();
    assert_eq!(processor.cached_analyses(), 2);
    let expected = spec::effects::analyze_recolour(spec::effects::AnalyzeRequest {
        version: 1,
        source: Source {
            width: WIDTH,
            height: HEIGHT,
            data: &data,
        },
        effects: &spec::effects::decode_effects(&json!([levels_gamma(2.0)]).to_string()).unwrap(),
        context: spec::effects::EffectContext {
            palette: &PALETTE,
            space: Some(ditherette_wasm::spec::contract::request::WorkingSpace::Oklab),
        },
    })
    .unwrap();
    assert_eq!(
        serde_json::to_value(&recipe).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );

    // Process v2 with the recolour step matches the reference and reuses the analysis too.
    let recipe_v2 = spec::effects::decode_recipe_v2(
        &json!({ "version": 2, "effects": recolour_chain(levels_gamma(2.0), levels_gamma(1.7)),
            "output": { "width": 7, "height": 5, "resize": { "algorithm": "area" } },
            "alpha": { "mode": "preserve", "threshold": 127.5 }, "match": "oklab-euclidean",
            "dither": { "family": "none" } })
        .to_string(),
    )
    .unwrap();
    let expected = spec::effects::process(spec::effects::ProcessRequestV2 {
        source: Source {
            width: WIDTH,
            height: HEIGHT,
            data: &data,
        },
        palette: &PALETTE,
        recipe: &recipe_v2,
    })
    .unwrap();
    let effects = prod_effects::decode_effects(
        &recolour_chain(levels_gamma(2.0), levels_gamma(1.7)).to_string(),
    )
    .unwrap();
    let mut io = Io {
        data: &data,
        events: None,
    };
    let actual = processor
        .process_effects(
            ProcessRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                palette: &PALETTE,
                recipe: prod_recipe(&recipe_v2),
            },
            &effects,
            &mut io,
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(processor.cached_analyses(), 2);
}

#[test]
fn warm_process_v2_never_reuses_a_stale_effected_source() {
    let first = ramp();
    let mut second = ramp();
    second[4] ^= 0x40;
    let recipe = recipe(json!({ "family": "none" }));
    let spec_for = |data: &[u8], recipe: &spec::effects::RecipeV2| {
        spec::effects::process(spec::effects::ProcessRequestV2 {
            source: Source {
                width: WIDTH,
                height: HEIGHT,
                data,
            },
            palette: &PALETTE,
            recipe,
        })
        .unwrap()
    };
    let mut other = recipe.clone();
    other.effects[0] = spec::effects::decode_effects(&json!([levels_gamma(3.0)]).to_string())
        .unwrap()
        .remove(0);
    let mut processor = processor();
    let run = |processor: &mut Processor, data: &[u8], recipe: &spec::effects::RecipeV2| {
        let effects =
            prod_effects::decode_effects(&serde_json::to_string(&recipe.effects).unwrap()).unwrap();
        let mut io = Io { data, events: None };
        processor
            .process_effects(
                ProcessRequest {
                    source_width: WIDTH,
                    source_height: HEIGHT,
                    palette: &PALETTE,
                    recipe: prod_recipe(recipe),
                },
                &effects,
                &mut io,
            )
            .unwrap()
    };
    for (data, recipe) in [
        (&first, &recipe),
        (&first, &recipe),
        (&second, &recipe),
        (&second, &other),
        (&first, &other),
    ] {
        assert_eq!(run(&mut processor, data, recipe), spec_for(data, recipe));
    }
    // Another call in between replaces the process snapshot; v2 must not trust it.
    let mut io = Io {
        data: &second,
        events: None,
    };
    processor
        .process(
            ProcessRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                palette: &PALETTE,
                recipe: prod_recipe(&recipe),
            },
            &mut io,
        )
        .unwrap();
    assert_eq!(
        run(&mut processor, &first, &other),
        spec_for(&first, &other)
    );
}

#[test]
fn recolour_calls_fit_exactly_their_preflighted_budget() {
    let data = ramp();
    let chain = recolour_chain(levels_gamma(1.2), levels_gamma(0.8));
    let effects = prod_effects::decode_effects(&chain.to_string()).unwrap();
    let prefix = prod_effects::decode_effects(&json!([levels_gamma(1.2)]).to_string()).unwrap();
    let context = prod_effects::EffectContext {
        palette: &PALETTE,
        space: Some(WorkingSpace::Oklab),
        analyses: None,
    };
    let mask: Vec<prod_effects::mask::MaskCurve> = serde_json::from_value(json!([
        { "x": { "model": "oklch", "channel": "lightness" }, "points": [[0, 1], [1, 0]] }
    ]))
    .unwrap();
    let calls: [&dyn Fn(&mut Processor) -> Result<(), Failure>; 3] = [
        &|processor| {
            let mut io = Io {
                data: &data,
                events: None,
            };
            let request = EffectsRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                effects: &effects,
                context,
            };
            processor.apply_effects(request, &mut io).map(drop)
        },
        &|processor| {
            let mut io = Io {
                data: &data,
                events: None,
            };
            let request = EffectsRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                effects: &prefix,
                context,
            };
            processor.analyze_recolour(request, &mut io).map(drop)
        },
        &|processor| {
            let mut io = Io {
                data: &data,
                events: None,
            };
            let request = EffectsRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                effects: &effects,
                context,
            };
            processor.effect_mask(request, &mask, &mut io).map(drop)
        },
    ];
    for call in calls {
        let mut generous = processor();
        call(&mut generous).unwrap();
        let peak = generous.peak_capacity_bytes();
        let mut exact = Processor::new(peak, 0).unwrap();
        call(&mut exact).unwrap();
        let mut short = Processor::new(peak - 1, 0).unwrap();
        assert_eq!(
            call(&mut short).unwrap_err(),
            Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
        );
    }
}

/// The reference mask: each pixel's strength on the carrier after `effects`, as grey bytes.
fn spec_mask(
    data: &[u8],
    width: u32,
    effects: &serde_json::Value,
    mask: &serde_json::Value,
) -> Vec<u8> {
    let steps = spec::effects::decode_effects(&effects.to_string()).unwrap();
    let mask: Vec<spec::effects::mask::MaskCurve> = serde_json::from_value(mask.clone()).unwrap();
    let dimensions = ImageDimensions::new(width, (data.len() / 4) as u32 / width).unwrap();
    let source = ditherette_wasm::image::ImageView::packed(data, dimensions).unwrap();
    let mut image = spec::effects::EffectImage::from_rgba8(source);
    let context = spec::effects::EffectContext {
        palette: &PALETTE,
        space: Some(spec::contract::request::WorkingSpace::Oklab),
    };
    spec::effects::apply_chain(&mut image, &steps, &context).unwrap();
    spec::effects::mask::strengths(&mask, &image)
        .iter()
        .zip(&image.alpha)
        .flat_map(|(strength, &alpha)| {
            let grey = (strength * 255.0).round() as u8;
            [grey, grey, grey, alpha]
        })
        .collect()
}

fn prod_mask(
    processor: &mut Processor,
    data: &[u8],
    width: u32,
    effects: &serde_json::Value,
    mask: &serde_json::Value,
) -> Vec<u8> {
    let steps = prod_effects::decode_effects(&effects.to_string()).unwrap();
    let mask: Vec<prod_effects::mask::MaskCurve> = serde_json::from_value(mask.clone()).unwrap();
    let mut io = Io { data, events: None };
    let request = EffectsRequest {
        source_width: width,
        source_height: (data.len() / 4) as u32 / width,
        effects: &steps,
        context: prod_effects::EffectContext {
            palette: &PALETTE,
            space: Some(WorkingSpace::Oklab),
            analyses: None,
        },
    };
    processor.effect_mask(request, &mask, &mut io).unwrap()
}

#[test]
fn effect_masks_match_the_reference_on_the_unclipped_carrier() {
    let data = ramp();
    let hue = |x: f32| json!({ "x": { "model": "oklch", "channel": "hue" }, "points": [[0, x], [0.5, 1], [1, x]] });
    let shadows = json!({ "x": { "model": "oklch", "channel": "lightness" }, "points": [[0, 1], [0.6, 0.2], [1, 0]] });
    let grid = json!({
        "x": { "model": "oklch", "channel": "hue" },
        "x2": { "model": "oklch", "channel": "chroma" },
        "grid": { "columns": [0, 0.5], "rows": [0, 1], "values": [[1, 0], [1, 0.5]] },
    });
    let boost = json!({ "effect": "exposure", "enabled": true, "stops": 1.5 });
    let saturate = json!({ "effect": "hue-saturation", "enabled": true, "hue": 20, "saturation": 0.6, "lightness": 0 });
    let chains = [
        json!([]),
        chain(),
        json!([chain()[0], saturate, boost]),
        recolour_chain(levels_gamma(1.2), boost),
    ];
    let masks = [json!([]), json!([hue(0.0)]), json!([shadows, grid])];
    let mut processor = processor();
    for effects in &chains {
        for mask in &masks {
            assert_eq!(
                prod_mask(&mut processor, &data, WIDTH, effects, mask),
                spec_mask(&data, WIDTH, effects, mask),
                "{effects} {mask}"
            );
        }
    }
}

#[test]
fn effect_masks_read_colour_beyond_white() {
    // Brightness 1 lifts this colour to about [1.5, 1.25, 1.13]: still chromatic, though it
    // clips to white. A mask holding back every hue sees the colour, not white.
    let data = [128, 64, 32, 200];
    let effects = json!([{ "effect": "brightness-contrast", "enabled": true, "brightness": 1, "contrast": 0 }]);
    let mask = json!([{ "x": { "model": "oklch", "channel": "hue" }, "points": [[0, 0], [1, 0]] }]);
    let expected = spec_mask(&data, 1, &effects, &mask);
    assert_eq!(expected, [0, 0, 0, 200]);
    assert_eq!(
        prod_mask(&mut processor(), &data, 1, &effects, &mask),
        expected
    );
}

#[test]
fn invalid_masks_fail_at_the_mask_path() {
    let data = ramp();
    let effects = prod_effects::decode_effects(&chain().to_string()).unwrap();
    let mask: Vec<prod_effects::mask::MaskCurve> = serde_json::from_value(json!([
        { "x": { "model": "oklch", "channel": "hue" }, "points": [[0, 0], [1, 1]] }
    ]))
    .unwrap();
    let mut io = Io {
        data: &data,
        events: None,
    };
    let request = EffectsRequest {
        source_width: WIDTH,
        source_height: HEIGHT,
        effects: &effects,
        context: prod_effects::EffectContext::default(),
    };
    assert_eq!(
        processor()
            .effect_mask(request, &mask, &mut io)
            .unwrap_err(),
        Failure::new(ErrorCode::InvalidSettings, ErrorPath::Mask)
    );
}

fn fit_chain(before: serde_json::Value, after: serde_json::Value) -> serde_json::Value {
    let mut chain = vec![before];
    chain.push(
        json!({ "effect": "palette-fit", "enabled": true, "look": "fitted",
        "space": "oklab", "strength": 0.8, "curves": null }),
    );
    chain.push(after);
    serde_json::Value::Array(chain)
}

#[test]
fn palette_fit_analyses_dedupe_within_a_call() {
    let data = ramp();
    let mut processor = processor();
    let run = |processor: &mut Processor, chain: &serde_json::Value| {
        let effects = prod_effects::decode_effects(&chain.to_string()).unwrap();
        let mut io = Io {
            data: &data,
            events: None,
        };
        processor
            .apply_effects(
                EffectsRequest {
                    source_width: WIDTH,
                    source_height: HEIGHT,
                    effects: &effects,
                    context: prod_effects::EffectContext {
                        palette: &PALETTE,
                        space: None,
                        analyses: None,
                    },
                },
                &mut io,
            )
            .unwrap()
    };
    let expected = {
        let spec_steps = spec::effects::decode_effects(
            &fit_chain(levels_gamma(1.2), levels_gamma(0.9)).to_string(),
        )
        .unwrap();
        spec::effects::apply_effects(spec::effects::EffectsRequest {
            version: 1,
            source: Source {
                width: WIDTH,
                height: HEIGHT,
                data: &data,
            },
            effects: &spec_steps,
            context: spec::effects::EffectContext {
                palette: &PALETTE,
                space: None,
            },
        })
        .unwrap()
        .into_vec()
    };
    assert_eq!(
        run(
            &mut processor,
            &fit_chain(levels_gamma(1.2), levels_gamma(0.9))
        ),
        expected,
        "spec parity"
    );
    // Fit curves are call-scoped working memory: nothing publishes, so repeated calls
    // re-analyse deterministically to the same bytes.
    assert_eq!(processor.cached_analyses(), 0);
    let expected = {
        let spec_steps = spec::effects::decode_effects(
            &fit_chain(levels_gamma(1.2), levels_gamma(1.7)).to_string(),
        )
        .unwrap();
        spec::effects::apply_effects(spec::effects::EffectsRequest {
            version: 1,
            source: Source {
                width: WIDTH,
                height: HEIGHT,
                data: &data,
            },
            effects: &spec_steps,
            context: spec::effects::EffectContext {
                palette: &PALETTE,
                space: None,
            },
        })
        .unwrap()
        .into_vec()
    };
    assert_eq!(
        run(
            &mut processor,
            &fit_chain(levels_gamma(1.2), levels_gamma(1.7)),
        ),
        expected,
        "a second call still matches the reference"
    );
    assert_eq!(processor.cached_analyses(), 0);

    // Standalone analysis of the same prefix is a hit and matches the reference's curves.
    let prefix = prod_effects::decode_effects(&json!([levels_gamma(2.0)]).to_string()).unwrap();
    let mut io = Io {
        data: &data,
        events: None,
    };
    let curves = processor
        .analyze_palette_fit(
            EffectsRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                effects: &prefix,
                context: prod_effects::EffectContext {
                    palette: &PALETTE,
                    space: None,
                    analyses: None,
                },
            },
            prod_effects::palette_fit::FitSpace::Oklab,
            prod_effects::palette_fit::FitLook::Fitted,
            &mut io,
        )
        .unwrap();
    assert_eq!(processor.cached_analyses(), 0);
    let expected = spec::effects::analyze_palette_fit(spec::effects::AnalyzePaletteFitRequest {
        version: 1,
        source: Source {
            width: WIDTH,
            height: HEIGHT,
            data: &data,
        },
        effects: &spec::effects::decode_effects(&json!([levels_gamma(2.0)]).to_string()).unwrap(),
        context: spec::effects::EffectContext {
            palette: &PALETTE,
            space: None,
        },
        space: spec::effects::palette_fit::FitSpace::Oklab,
        look: spec::effects::palette_fit::FitLook::Fitted,
    })
    .unwrap();
    assert_eq!(
        serde_json::to_value(&curves).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );

    // Process v2 with the palette-fit step matches the reference.
    let recipe_v2 = spec::effects::decode_recipe_v2(
        &json!({ "version": 2, "effects": fit_chain(levels_gamma(2.0), levels_gamma(1.7)),
            "output": { "width": 7, "height": 5, "resize": { "algorithm": "area" } },
            "alpha": { "mode": "preserve", "threshold": 127.5 }, "match": "oklab-euclidean",
            "dither": { "family": "none" } })
        .to_string(),
    )
    .unwrap();
    let expected = spec::effects::process(spec::effects::ProcessRequestV2 {
        source: Source {
            width: WIDTH,
            height: HEIGHT,
            data: &data,
        },
        palette: &PALETTE,
        recipe: &recipe_v2,
    })
    .unwrap();
    let mut io = Io {
        data: &data,
        events: None,
    };
    let effects =
        prod_effects::decode_effects(&fit_chain(levels_gamma(2.0), levels_gamma(1.7)).to_string())
            .unwrap();
    let actual = processor
        .process_effects(
            ProcessRequest {
                source_width: WIDTH,
                source_height: HEIGHT,
                palette: &PALETTE,
                recipe: prod_recipe(&recipe_v2),
            },
            &effects,
            &mut io,
        )
        .unwrap();
    assert_eq!(actual.indices.data(), expected.indices.data());
    assert_eq!(processor.cached_analyses(), 0);
}
