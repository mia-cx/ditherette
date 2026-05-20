use std::{env, fs, hint::black_box, path::PathBuf, sync::OnceLock};

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
use ditherette_wasm::{
    image::{rgba, ImageDimensions},
    resize::{
        bilinear::{resize_rgba_bilinear_2_into, resize_rgba_bilinear_reference},
        resize_rgba_bilinear, resize_rgba_bilinear_into,
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

const BILINEAR_FILTER: &str = "bilinear";
const BILINEAR2_FILTER: &str = "bilinear_2";

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

    if active_filter_is(BILINEAR2_FILTER) {
        bench_repeated_bilinear2(criterion, fixture, Scale::new("0.95x", 0.95));
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

    if active_filter_is(BILINEAR_FILTER) {
        bench_resize_into(
            &mut group,
            BILINEAR_FILTER,
            fixture,
            output_dimensions,
            output_byte_len,
            resize_rgba_bilinear_into,
        );
    }

    if active_filter_is(BILINEAR2_FILTER) {
        bench_resize_into(
            &mut group,
            BILINEAR2_FILTER,
            fixture,
            output_dimensions,
            output_byte_len,
            resize_rgba_bilinear_2_into,
        );
    }

    group.finish();
}

fn active_filter_is(filter: &str) -> bool {
    env::var("RESIZE_FILTER").map_or(true, |active_filter| active_filter == filter)
}

fn bench_resize_into(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    name: &str,
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    resize: fn(
        &[u8],
        ImageDimensions,
        ImageDimensions,
        &mut [u8],
    ) -> Result<(), ditherette_wasm::error::ProcessingError>,
) {
    group.bench_function(name, |bencher| {
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            resize(
                black_box(&fixture.rgba),
                fixture.dimensions,
                output_dimensions,
                black_box(&mut output_rgba),
            )
            .unwrap();
            black_box(&output_rgba);
        });
    });
}

fn bench_repeated_bilinear2(criterion: &mut Criterion, fixture: &RgbaFixture, scale: Scale) {
    const RESIZES_PER_ITERATION: usize = 8;

    let output_dimensions = scale.dimensions_for(fixture.dimensions);
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
    assert_resize_variants_match_baseline(fixture, output_dimensions, output_byte_len);

    let group_name = format!(
        "resize_bilinear_repeated/celeste_rgba/{}-{}x{}",
        scale.label,
        output_dimensions.width(),
        output_dimensions.height()
    );
    let mut group = criterion.benchmark_group(group_name);

    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(
        (output_byte_len * RESIZES_PER_ITERATION) as u64,
    ));

    group.bench_function("bilinear_2", |bencher| {
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            for _ in 0..RESIZES_PER_ITERATION {
                resize_rgba_bilinear_2_into(
                    black_box(&fixture.rgba),
                    fixture.dimensions,
                    output_dimensions,
                    black_box(&mut output_rgba),
                )
                .unwrap();
            }
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

    output_rgba.fill(0xA5);
    resize_rgba_bilinear_2_into(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .expect("bilinear_2 resize should succeed");
    assert_bytes_equal("bilinear_2", &output_rgba, &expected);
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
    let fixture_dir = benchmark_fixture_dir();

    if let Ok(value) = env::var("RESIZE_FIXTURES") {
        if let Some(path) = value
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    fixture_dir.join(path)
                }
            })
            .find(|path| path.exists())
        {
            return path;
        }
    }

    let preferred = fixture_dir.join("Celeste_box_art.png");
    if preferred.exists() {
        return preferred;
    }

    fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| {
            panic!(
                "failed to read benchmark fixture dir {}: {error}",
                fixture_dir.display()
            )
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| matches_fixture_extension(path))
        .unwrap_or_else(|| panic!("no PNG/JPEG fixtures found in {}", fixture_dir.display()))
}

fn benchmark_fixture_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(|repo_root| repo_root.join("benchmark-fixtures"))
        .expect("crate should live under crates/ditherette-wasm")
}

fn matches_fixture_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg"
            )
        })
        .unwrap_or(false)
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
