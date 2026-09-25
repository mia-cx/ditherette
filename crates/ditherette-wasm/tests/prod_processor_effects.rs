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
            request::RecipeV1,
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
        // Twice: the second call exercises the effected snapshot comparison and warm caches.
        for _ in 0..2 {
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
            assert!(
                effects_at.is_some_and(|index| index > 0),
                "{dither}: {events:?}"
            );
        }
    }
}

#[test]
fn disabled_chains_are_exactly_v1_process() {
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
    let v2 = processor()
        .process_effects(request, &effects, &mut io)
        .unwrap();
    let v1 = processor().process(request, &mut io).unwrap();
    assert_eq!(v2, v1);
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
