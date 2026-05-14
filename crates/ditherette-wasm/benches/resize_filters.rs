use std::{env, hint::black_box, path::PathBuf, sync::OnceLock};

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
#[cfg(feature = "tiling")]
use ditherette_wasm::resize::cpu_tiling::{plan_row_bands, DEFAULT_ROW_BAND_TILING};
#[cfg(feature = "tiling")]
use ditherette_wasm::resize::{
    area::resize_rgba_area_tiling_into, nearest::resize_rgba_nearest_tiling_into,
};
use ditherette_wasm::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        antialias::antialias_rgba_box3_reference_into, antialias_rgba_box3_into,
        area::resize_rgba_area_reference_into, bicubic::resize_rgba_bicubic_reference_into,
        bilinear::resize_rgba_bilinear_reference_into, lanczos::resize_rgba_lanczos_reference_into,
        nearest::resize_rgba_nearest_reference_into, r#box::resize_rgba_box_reference_into,
        resize_rgba_bicubic_into, resize_rgba_bilinear_into, resize_rgba_box_into,
        resize_rgba_lanczos2_into, resize_rgba_lanczos2_scale_aware_into,
        resize_rgba_lanczos3_into, resize_rgba_lanczos3_scale_aware_into,
        resize_rgba_trilinear_into, trilinear::resize_rgba_trilinear_reference_into,
    },
};
use image::{imageops::FilterType, ImageBuffer, ImageReader, RgbaImage};

#[cfg(not(feature = "tiling"))]
use ditherette_wasm::resize::{
    area::resize_rgba_area_scalar_into as resize_rgba_area_bench_into,
    nearest::resize_rgba_nearest_scalar_into as resize_rgba_nearest_bench_into,
};
#[cfg(feature = "tiling")]
use resize_rgba_area_tiling_into as resize_rgba_area_bench_into;
#[cfg(feature = "tiling")]
use resize_rgba_nearest_tiling_into as resize_rgba_nearest_bench_into;

fn resize_rgba_lanczos2_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        2.0,
        false,
    )
}

fn resize_rgba_lanczos2_scale_aware_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        2.0,
        true,
    )
}

fn resize_rgba_lanczos3_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        3.0,
        false,
    )
}

fn resize_rgba_lanczos3_scale_aware_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        3.0,
        true,
    )
}

const RESIZE_SCALES: [Scale; 6] = [
    Scale::new("2x", 2.0),
    Scale::new("0.95x", 0.95),
    Scale::new("0.75x", 0.75),
    Scale::new("0.5x", 0.5),
    Scale::new("0.25x", 0.25),
    Scale::new("0.125x", 0.125),
];

const RESIZE_FILTERS: [ResizeFilter; 20] = [
    ResizeFilter::new("nearest", resize_rgba_nearest_bench_into),
    ResizeFilter::new("nearest_reference", resize_rgba_nearest_reference_into),
    ResizeFilter::new("bilinear", resize_rgba_bilinear_into),
    ResizeFilter::new("bilinear_reference", resize_rgba_bilinear_reference_into),
    ResizeFilter::new("trilinear", resize_rgba_trilinear_into),
    ResizeFilter::new("trilinear_reference", resize_rgba_trilinear_reference_into),
    ResizeFilter::new("bicubic", resize_rgba_bicubic_into),
    ResizeFilter::new("bicubic_reference", resize_rgba_bicubic_reference_into),
    ResizeFilter::new("lanczos2", resize_rgba_lanczos2_into),
    ResizeFilter::new("lanczos2_reference", resize_rgba_lanczos2_reference_into),
    ResizeFilter::new(
        "lanczos2_scale_aware",
        resize_rgba_lanczos2_scale_aware_into,
    ),
    ResizeFilter::new(
        "lanczos2_scale_aware_reference",
        resize_rgba_lanczos2_scale_aware_reference_into,
    ),
    ResizeFilter::new("lanczos3", resize_rgba_lanczos3_into),
    ResizeFilter::new("lanczos3_reference", resize_rgba_lanczos3_reference_into),
    ResizeFilter::new(
        "lanczos3_scale_aware",
        resize_rgba_lanczos3_scale_aware_into,
    ),
    ResizeFilter::new(
        "lanczos3_scale_aware_reference",
        resize_rgba_lanczos3_scale_aware_reference_into,
    ),
    ResizeFilter::new("area", resize_rgba_area_bench_into),
    ResizeFilter::new("area_reference", resize_rgba_area_reference_into),
    ResizeFilter::new("box", resize_rgba_box_into),
    ResizeFilter::new("box_reference", resize_rgba_box_reference_into),
];

const NEAREST_ANTIALIAS_FILTERS: [NearestAntialiasFilter; 2] = [
    NearestAntialiasFilter::new(
        "nearest_aa",
        resize_rgba_nearest_bench_into,
        antialias_rgba_box3_into,
    ),
    NearestAntialiasFilter::new(
        "nearest_aa_reference",
        resize_rgba_nearest_reference_into,
        antialias_rgba_box3_reference_into,
    ),
];

const IMAGE_RESIZE_FILTERS: [ImageResizeFilter; 3] = [
    ImageResizeFilter::new("bilinear_image", FilterType::Triangle),
    ImageResizeFilter::new("bicubic_image", FilterType::CatmullRom),
    ImageResizeFilter::new("lanczos3_image", FilterType::Lanczos3),
];

static CELESTE_FIXTURE: OnceLock<RgbaFixture> = OnceLock::new();

/// Benchmarks resize filters against their side-by-side reference IDs.
///
/// The PNG is decoded before Criterion measures each kernel. These timings cover
/// Rust resize work over an already-materialized RGBA buffer, not browser decode
/// or JavaScript/Wasm boundary costs.
fn resize_filter_variants(criterion: &mut Criterion) {
    let fixture = CELESTE_FIXTURE.get_or_init(load_celeste_fixture);
    let selected_filter = requested_resize_filter();

    for scale in RESIZE_SCALES {
        bench_scale(criterion, fixture, scale, selected_filter.as_deref());
    }
}

fn bench_scale(
    criterion: &mut Criterion,
    fixture: &RgbaFixture,
    scale: Scale,
    selected_filter: Option<&str>,
) {
    let output_dimensions = scale.dimensions_for(fixture.dimensions);
    report_tiling_plan(
        scale,
        fixture.dimensions,
        output_dimensions,
        selected_filter,
    );
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
    assert_resize_filters_succeed(fixture, output_dimensions, output_byte_len, selected_filter);

    let group_name = format!(
        "resize_filters/celeste_rgba/{}-{}x{}",
        scale.label,
        output_dimensions.width(),
        output_dimensions.height()
    );
    let mut group = criterion.benchmark_group(group_name);

    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(output_byte_len as u64));

    for filter in RESIZE_FILTERS {
        if should_measure_filter(filter.name, selected_filter) {
            bench_resize_filter(
                &mut group,
                fixture,
                output_dimensions,
                output_byte_len,
                filter,
            );
        }
    }

    for filter in NEAREST_ANTIALIAS_FILTERS {
        if should_measure_filter(filter.name, selected_filter) {
            bench_nearest_antialias_filter(
                &mut group,
                fixture,
                output_dimensions,
                output_byte_len,
                filter,
            );
        }
    }

    if should_bench_antialias(selected_filter) {
        bench_antialias_filter(&mut group, fixture, output_dimensions, output_byte_len);
    }

    group.finish();
}

#[cfg(feature = "tiling")]
fn report_tiling_plan(
    scale: Scale,
    _source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    selected_filter: Option<&str>,
) {
    let plan = plan_row_bands(
        output_dimensions.width() as usize,
        output_dimensions.height() as usize,
        DEFAULT_ROW_BAND_TILING,
    );
    let enabled = plan.band_count > 1;

    eprintln!(
        "tiling {} {}x{} filter={} enabled={} available_logical_threads={} worker_count={} band_count={} tile={}x{} min_rows_per_band={} min_parallel_output_pixels={} min_pixels_per_band={} max_workers={}",
        scale.label,
        output_dimensions.width(),
        output_dimensions.height(),
        selected_filter.unwrap_or("all"),
        enabled,
        plan.available_logical_threads,
        plan.worker_count,
        plan.band_count,
        plan.output_width,
        plan.band_height,
        plan.min_rows_per_band,
        plan.min_parallel_output_pixels,
        plan.min_pixels_per_band,
        plan.max_workers,
    );
}

#[cfg(not(feature = "tiling"))]
fn report_tiling_plan(
    _scale: Scale,
    _source_dimensions: ImageDimensions,
    _output_dimensions: ImageDimensions,
    _selected_filter: Option<&str>,
) {
}

fn bench_resize_filter(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    filter: ResizeFilter,
) {
    group.bench_function(filter.name, |bencher| {
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            (filter.resize_into)(
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

fn bench_nearest_antialias_filter(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    filter: NearestAntialiasFilter,
) {
    group.bench_function(filter.name, |bencher| {
        let mut resized_rgba = vec![0; output_byte_len];
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            (filter.resize_into)(
                black_box(&fixture.rgba),
                fixture.dimensions,
                output_dimensions,
                black_box(&mut resized_rgba),
            )
            .unwrap();
            (filter.antialias_into)(
                black_box(&resized_rgba),
                output_dimensions,
                black_box(&mut output_rgba),
            )
            .unwrap();
            black_box(&output_rgba);
        });
    });
}

fn bench_antialias_filter(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
) {
    group.bench_function("antialias", |bencher| {
        let resized_rgba = resized_nearest_fixture(fixture, output_dimensions, output_byte_len);
        let mut output_rgba = vec![0; output_byte_len];

        bencher.iter(|| {
            antialias_rgba_box3_into(
                black_box(&resized_rgba),
                output_dimensions,
                black_box(&mut output_rgba),
            )
            .unwrap();
            black_box(&output_rgba);
        });
    });
}

fn assert_resize_filters_succeed(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    selected_filter: Option<&str>,
) {
    for filter in RESIZE_FILTERS {
        if !filter_matches_selection(filter.name, selected_filter) {
            continue;
        }

        let mut output_rgba = vec![0xA5; output_byte_len];
        (filter.resize_into)(
            &fixture.rgba,
            fixture.dimensions,
            output_dimensions,
            &mut output_rgba,
        )
        .unwrap_or_else(|error| panic!("{} resize should succeed: {error}", filter.name));
    }

    assert_selected_filters_match_references(
        fixture,
        output_dimensions,
        output_byte_len,
        selected_filter,
    );
    assert_nearest_antialias_filters_succeed(
        fixture,
        output_dimensions,
        output_byte_len,
        selected_filter,
    );
    assert_nearest_antialias_matches_reference(
        fixture,
        output_dimensions,
        output_byte_len,
        selected_filter,
    );

    for filter in IMAGE_RESIZE_FILTERS {
        if !filter_matches_selection(filter.name, selected_filter) {
            continue;
        }

        assert_matches_image_filter(fixture, output_dimensions, output_byte_len, filter);
    }

    if should_bench_antialias(selected_filter) {
        let resized_rgba = resized_nearest_fixture(fixture, output_dimensions, output_byte_len);
        let mut antialias_rgba = vec![0xA5; output_byte_len];
        let mut antialias_reference_rgba = vec![0xA5; output_byte_len];
        antialias_rgba_box3_into(&resized_rgba, output_dimensions, &mut antialias_rgba)
            .expect("antialias should succeed");
        antialias_rgba_box3_reference_into(
            &resized_rgba,
            output_dimensions,
            &mut antialias_reference_rgba,
        )
        .expect("antialias_reference should succeed");
        report_byte_equality(
            output_dimensions,
            "antialias",
            "antialias_reference",
            &antialias_rgba,
            &antialias_reference_rgba,
        );
    }
}

fn assert_nearest_antialias_filters_succeed(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    selected_filter: Option<&str>,
) {
    for filter in NEAREST_ANTIALIAS_FILTERS {
        if !filter_matches_selection(filter.name, selected_filter) {
            continue;
        }

        nearest_antialias_for_check(fixture, output_dimensions, output_byte_len, filter)
            .unwrap_or_else(|error| panic!("{} resize should succeed: {error}", filter.name));
    }
}

fn assert_nearest_antialias_matches_reference(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    selected_filter: Option<&str>,
) {
    if !filter_matches_selection("nearest_aa", selected_filter) {
        return;
    }

    let local_rgba = nearest_antialias_for_check(
        fixture,
        output_dimensions,
        output_byte_len,
        NEAREST_ANTIALIAS_FILTERS[0],
    )
    .expect("nearest_aa resize should succeed");
    let reference_rgba = nearest_antialias_for_check(
        fixture,
        output_dimensions,
        output_byte_len,
        NEAREST_ANTIALIAS_FILTERS[1],
    )
    .expect("nearest_aa_reference resize should succeed");

    report_byte_equality(
        output_dimensions,
        "nearest_aa",
        "nearest_aa_reference",
        &local_rgba,
        &reference_rgba,
    );
}

fn assert_selected_filters_match_references(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    selected_filter: Option<&str>,
) {
    for filter in RESIZE_FILTERS {
        if filter.name.ends_with("_reference")
            || !filter_matches_selection(filter.name, selected_filter)
        {
            continue;
        }

        let Some(reference_filter) = RESIZE_FILTERS
            .iter()
            .find(|candidate| candidate.name == format!("{}_reference", filter.name))
        else {
            continue;
        };

        let local_rgba = resize_for_check(fixture, output_dimensions, output_byte_len, filter);
        let reference_rgba = resize_for_check(
            fixture,
            output_dimensions,
            output_byte_len,
            *reference_filter,
        );

        report_byte_equality(
            output_dimensions,
            filter.name,
            reference_filter.name,
            &local_rgba,
            &reference_rgba,
        );
    }
}

fn assert_matches_image_filter(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    image_filter: ImageResizeFilter,
) {
    let output_rgba = image::imageops::resize(
        &fixture.image,
        output_dimensions.width(),
        output_dimensions.height(),
        image_filter.filter_type,
    )
    .into_raw();
    assert_eq!(
        output_rgba.len(),
        output_byte_len,
        "{} output length should match requested dimensions",
        image_filter.name
    );

    let reference_name = format!("{}_reference", base_filter_name(image_filter.name));
    let reference_filter = RESIZE_FILTERS
        .iter()
        .find(|filter| filter.name == reference_name)
        .unwrap_or_else(|| {
            panic!(
                "{} should have a matching local reference",
                image_filter.name
            )
        });
    let reference_rgba = resize_for_check(
        fixture,
        output_dimensions,
        output_byte_len,
        *reference_filter,
    );

    report_byte_equality(
        output_dimensions,
        reference_filter.name,
        image_filter.name,
        &reference_rgba,
        &output_rgba,
    );
}

fn nearest_antialias_for_check(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    filter: NearestAntialiasFilter,
) -> Result<Vec<u8>, ProcessingError> {
    let mut resized_rgba = vec![0xA5; output_byte_len];
    let mut output_rgba = vec![0xA5; output_byte_len];
    (filter.resize_into)(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut resized_rgba,
    )?;
    (filter.antialias_into)(&resized_rgba, output_dimensions, &mut output_rgba)?;
    Ok(output_rgba)
}

fn resize_for_check(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    filter: ResizeFilter,
) -> Vec<u8> {
    let mut output_rgba = vec![0xA5; output_byte_len];
    (filter.resize_into)(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .unwrap_or_else(|error| panic!("{} resize should succeed: {error}", filter.name));
    output_rgba
}

fn report_byte_equality(
    output_dimensions: ImageDimensions,
    left_name: &str,
    right_name: &str,
    left: &[u8],
    right: &[u8],
) {
    if let Some(mismatch) = first_mismatch(left, right) {
        eprintln!(
            "correctness {}x{}: {} differs from {} at byte {}: {}={} {}={}",
            output_dimensions.width(),
            output_dimensions.height(),
            left_name,
            right_name,
            mismatch.index,
            left_name,
            mismatch.left,
            right_name,
            mismatch.right
        );
        return;
    }

    eprintln!(
        "correctness {}x{}: {} matches {} exactly",
        output_dimensions.width(),
        output_dimensions.height(),
        left_name,
        right_name
    );
}

#[derive(Debug, Clone, Copy)]
struct ByteMismatch {
    index: usize,
    left: u8,
    right: u8,
}

fn first_mismatch(left: &[u8], right: &[u8]) -> Option<ByteMismatch> {
    left.iter()
        .zip(right.iter())
        .enumerate()
        .find(|(_, (left, right))| left != right)
        .map(|(index, (left, right))| ByteMismatch {
            index,
            left: *left,
            right: *right,
        })
}

fn requested_resize_filter() -> Option<String> {
    let filter = env::var("RESIZE_FILTER").ok()?;
    if filter.is_empty() {
        return None;
    }

    if filter == "antialias"
        || NEAREST_ANTIALIAS_FILTERS
            .iter()
            .any(|candidate| base_filter_name(candidate.name) == filter)
        || RESIZE_FILTERS
            .iter()
            .any(|candidate| base_filter_name(candidate.name) == filter)
        || IMAGE_RESIZE_FILTERS
            .iter()
            .any(|candidate| base_filter_name(candidate.name) == filter)
    {
        return Some(filter);
    }

    panic!("unknown resize filter `{filter}`");
}

fn should_measure_filter(filter_name: &str, selected_filter: Option<&str>) -> bool {
    filter_matches_selection(filter_name, selected_filter) && !filter_name.ends_with("_reference")
}

fn filter_matches_selection(filter_name: &str, selected_filter: Option<&str>) -> bool {
    selected_filter.is_none_or(|selected| base_filter_name(filter_name) == selected)
}

fn should_bench_antialias(selected_filter: Option<&str>) -> bool {
    selected_filter.is_none_or(|selected| selected == "antialias")
}

fn base_filter_name(filter_name: &str) -> &str {
    filter_name
        .strip_suffix("_reference")
        .or_else(|| filter_name.strip_suffix("_image"))
        .unwrap_or(filter_name)
}

fn resized_nearest_fixture(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
) -> Vec<u8> {
    let mut output_rgba = vec![0; output_byte_len];
    resize_rgba_nearest_bench_into(
        &fixture.rgba,
        fixture.dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .expect("nearest resize should succeed before antialias bench");
    output_rgba
}

fn load_celeste_fixture() -> RgbaFixture {
    let fixture_path = fixture_path();
    let image = ImageReader::open(&fixture_path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", fixture_path.display()))
        .decode()
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", fixture_path.display()))
        .to_rgba8();

    let dimensions = ImageDimensions::new(image.width(), image.height()).unwrap();

    let rgba = image.into_raw();
    let image = ImageBuffer::from_raw(dimensions.width(), dimensions.height(), rgba.clone())
        .expect("decoded RGBA fixture should match its dimensions");

    RgbaFixture {
        dimensions,
        image,
        rgba,
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
    image: RgbaImage,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct ResizeFilter {
    name: &'static str,
    resize_into: ResizeInto,
}

type ResizeInto =
    fn(&[u8], ImageDimensions, ImageDimensions, &mut [u8]) -> Result<(), ProcessingError>;
type AntialiasInto = fn(&[u8], ImageDimensions, &mut [u8]) -> Result<(), ProcessingError>;

impl ResizeFilter {
    const fn new(name: &'static str, resize_into: ResizeInto) -> Self {
        Self { name, resize_into }
    }
}

#[derive(Debug, Clone, Copy)]
struct NearestAntialiasFilter {
    name: &'static str,
    resize_into: ResizeInto,
    antialias_into: AntialiasInto,
}

impl NearestAntialiasFilter {
    const fn new(
        name: &'static str,
        resize_into: ResizeInto,
        antialias_into: AntialiasInto,
    ) -> Self {
        Self {
            name,
            resize_into,
            antialias_into,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ImageResizeFilter {
    name: &'static str,
    filter_type: FilterType,
}

impl ImageResizeFilter {
    const fn new(name: &'static str, filter_type: FilterType) -> Self {
        Self { name, filter_type }
    }
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

criterion_group!(benches, resize_filter_variants);
criterion_main!(benches);
