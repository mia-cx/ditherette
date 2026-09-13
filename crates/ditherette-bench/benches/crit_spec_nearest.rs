//! Criterion cross-check for the custom resize benchmark harness.

use std::{hint::black_box, path::Path, time::Duration};

use criterion::{criterion_group, Criterion, SamplingMode, Throughput};
use ditherette_bench::lease::{require_quiet, BenchmarkGuard};
use ditherette_bench_api::{
    BenchSubject, ResizeBenchSubject, ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams,
};

const RGBA_CHANNELS: usize = 4;
const DOWNSCALE_GROUP: &[f64] = &[
    0.13, 0.16, 0.19, 0.25, 0.33, 0.5, 0.75, 0.85, 0.9, 0.93, 0.95, 0.97, 0.98, 0.99,
];

fn criterion_spec_nearest(criterion: &mut Criterion) {
    let fixture = load_fixture("Celeste_Insta_selfie.png");
    let subject = spec_nearest_subject();

    for scale in DOWNSCALE_GROUP {
        let output_width = output_axis(fixture.width, *scale);
        let output_height = output_axis(fixture.height, *scale);
        let case = format!(
            "resize_filters/{}/{}x-{}x{}/nearest",
            fixture.name, scale, output_width, output_height
        );
        let mut output = vec![0; output_width as usize * output_height as usize * RGBA_CHANNELS];
        let mut group = criterion.benchmark_group(case);
        group.sample_size(100);
        group.measurement_time(Duration::from_secs(5));
        group.warm_up_time(Duration::from_secs(1));
        group.sampling_mode(SamplingMode::Flat);
        group.throughput(Throughput::Elements(
            u64::from(output_width) * u64::from(output_height),
        ));
        group.bench_function("spec:resize:nearest:scalar", |bencher| {
            bencher.iter(|| {
                (subject.resize_u8_rgba)(
                    ResizeInputU8Rgba {
                        data: black_box(&fixture.rgba),
                        width: fixture.width,
                        height: fixture.height,
                        row_stride_elements: fixture.width as usize * RGBA_CHANNELS,
                    },
                    ResizeOutputU8Rgba {
                        data: black_box(&mut output),
                        width: output_width,
                        height: output_height,
                        row_stride_elements: output_width as usize * RGBA_CHANNELS,
                    },
                    black_box(&ResizeParams::default()),
                )
                .expect("spec nearest benchmark should resize successfully");
                black_box(&output);
            });
        });
        group.finish();
    }
}

fn spec_nearest_subject() -> ResizeBenchSubject {
    ditherette_wasm::bench_subjects()
        .into_iter()
        .find_map(|subject| match subject {
            BenchSubject::Resize(subject)
                if subject.descriptor.id.as_str() == "spec:resize:nearest:scalar" =>
            {
                Some(subject)
            }
            _ => None,
        })
        .expect("spec nearest subject should be registered")
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
        name: name.trim_end_matches(".png").to_owned(),
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

fn output_axis(source: u32, scale: f64) -> u32 {
    ((f64::from(source) * scale).round() as u32).max(1)
}

criterion_group!(benches, criterion_spec_nearest);

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
