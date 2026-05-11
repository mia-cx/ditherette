use std::{hint::black_box, path::PathBuf, sync::OnceLock};

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use ditherette_wasm::{
    image::{rgba, ImageDimensions},
    resize::{
        bilinear::resize_rgba_bilinear_reference, resize_rgba_bilinear, resize_rgba_bilinear_into,
    },
};
use image::ImageReader;

const RESIZE_SCALES: [Scale; 6] = [
    Scale::new("2x", 2.0),
    Scale::new("0.95x", 0.95),
    Scale::new("0.75x", 0.75),
    Scale::new("0.5x", 0.5),
    Scale::new("0.25x", 0.25),
    Scale::new("0.125x", 0.125),
];

static CELESTE_FIXTURE: OnceLock<RgbaFixture> = OnceLock::new();

/// Benchmarks bilinear resize against the Celeste fixture.
///
/// The PNG is decoded before Criterion measures each kernel. These timings cover
/// Rust resize work over an already-materialized RGBA buffer, not browser decode
/// or JavaScript/Wasm boundary costs.
fn resize_bilinear_variants(criterion: &mut Criterion) {
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
        "resize_bilinear/celeste_rgba/{}-{}x{}",
        scale.label,
        output_dimensions.width(),
        output_dimensions.height()
    );
    let mut group = criterion.benchmark_group(group_name);

    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(output_byte_len as u64));

    group.bench_function("baseline", |bencher| {
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            resize_rgba_bilinear_into(
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
    let expected =
        resize_rgba_bilinear_reference(&fixture.rgba, fixture.dimensions, output_dimensions)
            .expect("reference bilinear resize should succeed");

    assert_bytes_equal(
        "allocating-api",
        &resize_rgba_bilinear(&fixture.rgba, fixture.dimensions, output_dimensions)
            .expect("allocating bilinear resize should succeed"),
        &expected,
    );

    let mut output_rgba = vec![0xA5; output_byte_len];
    resize_rgba_bilinear_into(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .expect("baseline bilinear resize should succeed");
    assert_bytes_equal("baseline", &output_rgba, &expected);
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

criterion_group!(benches, resize_bilinear_variants);
criterion_main!(benches);
