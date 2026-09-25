//! Criterion comparison of the effects reference and production copy.
//!
//! Each case checks that both implementations return identical bytes before timing.
//! Run through the lease helper during a quiet phase (see EXECUTION.md).

use std::{hint::black_box, path::Path, time::Duration};

use criterion::{criterion_group, Criterion, SamplingMode, Throughput};
use ditherette_bench::lease::{require_quiet, BenchmarkGuard};
use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::{contract::request::Source as ProdSource, effects as prod},
    spec::{contract::request::Source, effects as spec},
};
use serde_json::json;

const FIXTURES: &[&str] = &["Celeste_Insta_selfie.png", "Picking_at_thread.jpg"];
const PALETTE: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Color {
        rgb: [255, 255, 255],
    },
];

/// Named chains. Later effects add their representative chains here.
fn chains() -> Vec<(&'static str, serde_json::Value)> {
    let levels = |channel: &str, input: (f32, f32), gamma: f32, output: (f32, f32)| {
        json!({
            "effect": "levels", "enabled": true, "channel": channel,
            "input": { "black": input.0, "white": input.1 }, "gamma": gamma,
            "output": { "black": output.0, "white": output.1 },
        })
    };
    vec![
        (
            "levels",
            json!([levels("rgb", (0.05, 0.95), 1.4, (0.0, 1.0))]),
        ),
        (
            "levels-x3",
            json!([
                levels("rgb", (0.05, 0.95), 1.4, (0.0, 1.0)),
                levels("red", (0.0, 1.0), 1.0, (0.1, 0.9)),
                levels("blue", (0.2, 1.0), 0.8, (0.0, 1.0)),
            ]),
        ),
    ]
}

fn criterion_effects(criterion: &mut Criterion) {
    for name in FIXTURES {
        let fixture = load_fixture(name);
        for (chain, effects) in chains() {
            let json = effects.to_string();
            let spec_steps = spec::decode_effects(&json).expect("chain decodes");
            let prod_steps = prod::decode_effects(&json).expect("chain decodes");
            let run_spec = || {
                spec::apply_effects(spec::EffectsRequest {
                    version: 1,
                    source: Source {
                        width: fixture.width,
                        height: fixture.height,
                        data: black_box(&fixture.rgba),
                    },
                    effects: &spec_steps,
                    context: spec::EffectContext {
                        palette: &PALETTE,
                        space: None,
                    },
                })
                .expect("spec applies")
            };
            let run_prod = || {
                prod::apply_effects(prod::EffectsRequest {
                    version: 1,
                    source: ProdSource {
                        width: fixture.width,
                        height: fixture.height,
                        data: black_box(&fixture.rgba),
                    },
                    effects: &prod_steps,
                    context: prod::EffectContext {
                        palette: &PALETTE,
                        space: None,
                    },
                })
                .expect("prod applies")
            };
            assert_eq!(run_prod().data(), run_spec().data(), "{chain} on {name}");

            let mut group = criterion.benchmark_group(format!("effects/{chain}/{}", fixture.name));
            group.sample_size(20);
            group.measurement_time(Duration::from_secs(5));
            group.warm_up_time(Duration::from_secs(1));
            group.sampling_mode(SamplingMode::Flat);
            group.throughput(Throughput::Elements(
                u64::from(fixture.width) * u64::from(fixture.height),
            ));
            group.bench_function("spec", |bencher| bencher.iter(|| black_box(run_spec())));
            group.bench_function("prod", |bencher| bencher.iter(|| black_box(run_prod())));
            group.finish();
        }
    }
}

struct Fixture {
    name: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn load_fixture(name: &str) -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under crates/ditherette-bench")
        .join("benchmark-fixtures")
        .join(name);
    let image = image::open(&path)
        .unwrap_or_else(|error| panic!("failed to load fixture {}: {error}", path.display()))
        .to_rgba8();
    Fixture {
        name: name
            .rsplit_once('.')
            .map_or(name, |(stem, _)| stem)
            .to_owned(),
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

criterion_group!(benches, criterion_effects);

fn main() {
    let result = BenchmarkGuard::acquire().and_then(|guard| {
        require_quiet()?;
        benches();
        Criterion::default().configure_from_args().final_summary();
        drop(guard);
        Ok(())
    });
    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(5);
    }
}
