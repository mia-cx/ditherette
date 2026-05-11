use std::{hint::black_box, path::PathBuf, sync::OnceLock};

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use ditherette_wasm::{
    image::{rgba, ImageDimensions},
    resize::{
        nearest::{
            resize_rgba_nearest_fast_paths, resize_rgba_nearest_precomputed_offsets,
            resize_rgba_nearest_reference, resize_rgba_nearest_reference_into,
            resize_rgba_nearest_word_copy,
        },
        resize_rgba_nearest, resize_rgba_nearest_into,
    },
};
use image::ImageReader;

const RESIZE_SCALES: [Scale; 5] = [
    Scale::new("0.95x", 0.95),
    Scale::new("0.75x", 0.75),
    Scale::new("0.5x", 0.5),
    Scale::new("0.25x", 0.25),
    Scale::new("0.125x", 0.125),
];

static CELESTE_FIXTURE: OnceLock<RgbaFixture> = OnceLock::new();

/// Benchmarks nearest-neighbor resize variants against the Celeste fixture.
///
/// The PNG is decoded before Criterion measures each kernel. These timings cover
/// Rust resize work over an already-materialized RGBA buffer, not browser decode
/// or JavaScript/Wasm boundary costs.
fn resize_nearest_variants(criterion: &mut Criterion) {
    let fixture = CELESTE_FIXTURE.get_or_init(load_celeste_fixture);

    for scale in RESIZE_SCALES {
        bench_scale(criterion, fixture, scale);
    }
}

fn bench_scale(criterion: &mut Criterion, fixture: &RgbaFixture, scale: Scale) {
    let output_dimensions = scale.dimensions_for(fixture.dimensions);
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
    assert_resize_variants_match_baseline(fixture, output_dimensions, output_byte_len);

    let group_name = format!(
        "resize_nearest/celeste_rgba/{}-{}x{}",
        scale.label,
        output_dimensions.width(),
        output_dimensions.height()
    );
    let mut group = criterion.benchmark_group(group_name);

    // Flat sampling is better for millisecond-scale image kernels. Leave sample
    // count and measurement time to Criterion defaults or CLI flags such as
    // `--sample-size 10 --measurement-time 3`.
    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(output_byte_len as u64));

    group.bench_function("baseline", |bencher| {
        bencher.iter(|| {
            black_box(
                resize_rgba_nearest_reference(
                    black_box(&fixture.rgba),
                    fixture.dimensions,
                    output_dimensions,
                )
                .unwrap(),
            );
        });
    });

    group.bench_function("precomputed-offsets", |bencher| {
        bencher.iter(|| {
            black_box(
                resize_rgba_nearest_precomputed_offsets(
                    black_box(&fixture.rgba),
                    fixture.dimensions,
                    output_dimensions,
                )
                .unwrap(),
            );
        });
    });

    group.bench_function("fast-paths", |bencher| {
        bencher.iter(|| {
            black_box(
                resize_rgba_nearest_fast_paths(
                    black_box(&fixture.rgba),
                    fixture.dimensions,
                    output_dimensions,
                )
                .unwrap(),
            );
        });
    });

    group.bench_function("word-copy", |bencher| {
        bencher.iter(|| {
            black_box(
                resize_rgba_nearest_word_copy(
                    black_box(&fixture.rgba),
                    fixture.dimensions,
                    output_dimensions,
                )
                .unwrap(),
            );
        });
    });

    group.bench_function("into-reused-output", |bencher| {
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            resize_rgba_nearest_reference_into(
                black_box(&fixture.rgba),
                fixture.dimensions,
                output_dimensions,
                black_box(&mut output_rgba),
            )
            .unwrap();
            black_box(&output_rgba);
        });
    });

    group.bench_function("production", |bencher| {
        bencher.iter(|| {
            black_box(
                resize_rgba_nearest(
                    black_box(&fixture.rgba),
                    fixture.dimensions,
                    output_dimensions,
                )
                .unwrap(),
            );
        });
    });

    group.bench_function("production-into", |bencher| {
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            resize_rgba_nearest_into(
                black_box(&fixture.rgba),
                fixture.dimensions,
                output_dimensions,
                black_box(&mut output_rgba),
            )
            .unwrap();
            black_box(&output_rgba);
        });
    });

    group.finish();
}

fn assert_resize_variants_match_baseline(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
) {
    // Correctness checks run before Criterion starts sampling, so wrong fast
    // variants fail the benchmark without contaminating measured timings.
    let baseline =
        resize_rgba_nearest_reference(&fixture.rgba, fixture.dimensions, output_dimensions)
            .expect("baseline resize should succeed");

    assert_bytes_equal(
        "production",
        &resize_rgba_nearest(&fixture.rgba, fixture.dimensions, output_dimensions)
            .expect("production resize should succeed"),
        &baseline,
    );
    assert_bytes_equal(
        "precomputed-offsets",
        &resize_rgba_nearest_precomputed_offsets(
            &fixture.rgba,
            fixture.dimensions,
            output_dimensions,
        )
        .expect("precomputed-offsets resize should succeed"),
        &baseline,
    );
    assert_bytes_equal(
        "fast-paths",
        &resize_rgba_nearest_fast_paths(&fixture.rgba, fixture.dimensions, output_dimensions)
            .expect("fast-paths resize should succeed"),
        &baseline,
    );
    assert_bytes_equal(
        "word-copy",
        &resize_rgba_nearest_word_copy(&fixture.rgba, fixture.dimensions, output_dimensions)
            .expect("word-copy resize should succeed"),
        &baseline,
    );
    let mut output_rgba = vec![0xA5; output_byte_len];
    resize_rgba_nearest_reference_into(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .expect("into-reused-output resize should succeed");
    assert_bytes_equal("into-reused-output", &output_rgba, &baseline);

    output_rgba.fill(0xA5);
    resize_rgba_nearest_into(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .expect("production-into resize should succeed");
    assert_bytes_equal("production-into", &output_rgba, &baseline);
}

fn assert_bytes_equal(variant: &str, actual: &[u8], expected: &[u8]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{variant} output length should match baseline"
    );

    if let Some(index) = actual
        .iter()
        .zip(expected.iter())
        .position(|(actual, expected)| actual != expected)
    {
        panic!(
            "{variant} output differed from baseline at byte {index}: actual={} expected={}",
            actual[index], expected[index]
        );
    }
}

fn load_celeste_fixture() -> RgbaFixture {
    let fixture_path = fixture_path();
    let image = ImageReader::open(&fixture_path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", fixture_path.display()))
        .decode()
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", fixture_path.display()))
        .to_rgba8();

    let dimensions = ImageDimensions::new(image.width(), image.height()).unwrap();

    RgbaFixture {
        dimensions,
        rgba: image.into_raw(),
    }
}

fn fixture_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(|repo_root| repo_root.join("benchmark-fixtures/Celeste_box_art_full.png"))
        .expect("crate should live under crates/ditherette-wasm")
}

#[derive(Debug)]
struct RgbaFixture {
    dimensions: ImageDimensions,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct Scale {
    label: &'static str,
    multiplier: f64,
}

impl Scale {
    const fn new(label: &'static str, multiplier: f64) -> Self {
        Self { label, multiplier }
    }

    fn dimensions_for(self, source_dimensions: ImageDimensions) -> ImageDimensions {
        ImageDimensions::new(
            scaled_dimension(source_dimensions.width(), self.multiplier),
            scaled_dimension(source_dimensions.height(), self.multiplier),
        )
        .unwrap()
    }
}

fn scaled_dimension(source_dimension: u32, scale: f64) -> u32 {
    ((f64::from(source_dimension) * scale).floor() as u32).max(1)
}

criterion_group!(benches, resize_nearest_variants);
criterion_main!(benches);
