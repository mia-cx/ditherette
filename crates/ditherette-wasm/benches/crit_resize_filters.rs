use std::{
    env, fs,
    hint::black_box,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode, Throughput};
#[cfg(feature = "tiling")]
use ditherette_wasm::resize::cpu_tiling::{plan_row_bands, DEFAULT_ROW_BAND_TILING};
#[cfg(feature = "tiling")]
use ditherette_wasm::resize::{
    area::{resize_rgba_area_dynamic_tiling_plan, resize_rgba_area_tiling_into},
    bilinear::{resize_rgba_bilinear_dynamic_tiling_plan, resize_rgba_bilinear_tiling_into},
    nearest::{resize_rgba_nearest_dynamic_tiling_plan, resize_rgba_nearest_tiling_into},
};
use ditherette_wasm::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        antialias::antialias_rgba_box3_reference_into,
        antialias_rgba_box3_into,
        area::{resize_rgba_area_reference_into, resize_rgba_area_scalar_into},
        bicubic::{
            resize_rgba_bicubic_2_fixed_into, resize_rgba_bicubic_2_into,
            resize_rgba_bicubic_2_scale_aware_into, resize_rgba_bicubic_reference_into,
        },
        bilinear::{
            resize_rgba_bilinear_2_into, resize_rgba_bilinear_reference_into,
            resize_rgba_bilinear_scalar_into,
        },
        lanczos::resize_rgba_lanczos_reference_into,
        lanczos3::{
            resize_rgba_lanczos3_2_fixed_into, resize_rgba_lanczos3_2_into,
            resize_rgba_lanczos3_2_scale_aware_into,
        },
        nearest::{resize_rgba_nearest_reference_into, resize_rgba_nearest_scalar_into},
        r#box::resize_rgba_box_reference_into,
        resize_rgba_bicubic_into, resize_rgba_bilinear_into, resize_rgba_box_into,
        resize_rgba_lanczos2_into, resize_rgba_lanczos2_scale_aware_into,
        resize_rgba_lanczos3_into, resize_rgba_lanczos3_scale_aware_into,
        resize_rgba_trilinear_into,
        trilinear::resize_rgba_trilinear_reference_into,
    },
};
use image::{imageops::FilterType, ImageBuffer, ImageReader, RgbaImage};

#[cfg(not(feature = "tiling"))]
use resize_rgba_area_scalar_into as resize_rgba_area_bench_into;
#[cfg(feature = "tiling")]
use resize_rgba_area_tiling_into as resize_rgba_area_bench_into;
#[cfg(not(feature = "tiling"))]
use resize_rgba_nearest_scalar_into as resize_rgba_nearest_bench_into;
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

const UPSCALE_SCALES: [Scale; 1] = [Scale::new("2x", 2.0)];
const FRACTIONAL_DOWNSCALE_SCALES: [Scale; 5] = [
    Scale::new("0.95x", 0.95),
    Scale::new("0.875x", 0.875),
    Scale::new("0.8x", 0.8),
    Scale::new("0.75x", 0.75),
    Scale::new("0.625x", 0.625),
];
const EXACT_DOWNSCALE_SCALES: [Scale; 4] = [
    Scale::new("0.5x", 0.5),
    Scale::new("0.375x", 0.375),
    Scale::new("0.25x", 0.25),
    Scale::new("0.125x", 0.125),
];
const RESIZE_SCALES: [Scale; 10] = [
    UPSCALE_SCALES[0],
    FRACTIONAL_DOWNSCALE_SCALES[0],
    FRACTIONAL_DOWNSCALE_SCALES[1],
    FRACTIONAL_DOWNSCALE_SCALES[2],
    FRACTIONAL_DOWNSCALE_SCALES[3],
    FRACTIONAL_DOWNSCALE_SCALES[4],
    EXACT_DOWNSCALE_SCALES[0],
    EXACT_DOWNSCALE_SCALES[1],
    EXACT_DOWNSCALE_SCALES[2],
    EXACT_DOWNSCALE_SCALES[3],
];

const RESIZE_FILTERS: [ResizeFilter; 27] = [
    ResizeFilter::new("nearest", resize_rgba_nearest_bench_into),
    ResizeFilter::new("nearest_reference", resize_rgba_nearest_reference_into),
    ResizeFilter::new("bilinear", resize_rgba_bilinear_into),
    ResizeFilter::new("bilinear_2", resize_rgba_bilinear_2_into),
    ResizeFilter::new("bilinear_reference", resize_rgba_bilinear_reference_into),
    ResizeFilter::new("trilinear", resize_rgba_trilinear_into),
    ResizeFilter::new("trilinear_reference", resize_rgba_trilinear_reference_into),
    ResizeFilter::new("bicubic", resize_rgba_bicubic_into),
    ResizeFilter::new("bicubic_2", resize_rgba_bicubic_2_into),
    ResizeFilter::new("bicubic_2_fixed", resize_rgba_bicubic_2_fixed_into),
    ResizeFilter::new(
        "bicubic_2_scale_aware",
        resize_rgba_bicubic_2_scale_aware_into,
    ),
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
    ResizeFilter::new("lanczos3_2", resize_rgba_lanczos3_2_into),
    ResizeFilter::new("lanczos3_2_fixed", resize_rgba_lanczos3_2_fixed_into),
    ResizeFilter::new(
        "lanczos3_2_scale_aware",
        resize_rgba_lanczos3_2_scale_aware_into,
    ),
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

static BENCHMARK_FIXTURES: OnceLock<Vec<RgbaFixture>> = OnceLock::new();

/// Benchmarks resize filters against their side-by-side reference IDs.
///
/// The PNG is decoded before Criterion measures each kernel. These timings cover
/// Rust resize work over an already-materialized RGBA buffer, not browser decode
/// or JavaScript/Wasm boundary costs.
fn resize_filter_variants(criterion: &mut Criterion) {
    let fixtures = BENCHMARK_FIXTURES.get_or_init(load_benchmark_fixtures);

    let selected_scale_group = requested_resize_scale_group();

    if let Some(comparison) = requested_resize_comparison() {
        for fixture in fixtures {
            preflight_resize_comparison(fixture, &comparison, selected_scale_group);
            for &scale in selected_resize_scales(selected_scale_group) {
                bench_scale_comparison(criterion, fixture, scale, &comparison);
            }
        }
        return;
    }

    let selected_filter = requested_resize_filter();
    for fixture in fixtures {
        preflight_resize_filter_correctness(
            fixture,
            selected_filter.as_deref(),
            selected_scale_group,
        );

        for &scale in selected_resize_scales(selected_scale_group) {
            bench_scale(criterion, fixture, scale, selected_filter.as_deref());
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ScaleGroup {
    Upscale,
    FractionalDownscale,
    ExactDownscale,
}

fn selected_resize_scales(selected_scale_group: Option<ScaleGroup>) -> &'static [Scale] {
    match selected_scale_group {
        None => &RESIZE_SCALES,
        Some(ScaleGroup::Upscale) => &UPSCALE_SCALES,
        Some(ScaleGroup::FractionalDownscale) => &FRACTIONAL_DOWNSCALE_SCALES,
        Some(ScaleGroup::ExactDownscale) => &EXACT_DOWNSCALE_SCALES,
    }
}

fn requested_resize_scale_group() -> Option<ScaleGroup> {
    let value = env::var("RESIZE_SCALE_GROUP").ok()?;
    match value.as_str() {
        "upscale" => Some(ScaleGroup::Upscale),
        "fractional-downscale" => Some(ScaleGroup::FractionalDownscale),
        "exact-downscale" => Some(ScaleGroup::ExactDownscale),
        _ => panic!(
            "RESIZE_SCALE_GROUP must be one of: upscale, fractional-downscale, exact-downscale"
        ),
    }
}

fn preflight_resize_comparison(
    fixture: &RgbaFixture,
    comparison: &ResizeComparison,
    selected_scale_group: Option<ScaleGroup>,
) {
    for &scale in selected_resize_scales(selected_scale_group) {
        let output_dimensions = scale.dimensions_for(fixture.dimensions);
        let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
        assert_comparison_succeeds(fixture, output_dimensions, output_byte_len, comparison);
    }
}

fn preflight_resize_filter_correctness(
    fixture: &RgbaFixture,
    selected_filter: Option<&str>,
    selected_scale_group: Option<ScaleGroup>,
) {
    for &scale in selected_resize_scales(selected_scale_group) {
        let output_dimensions = scale.dimensions_for(fixture.dimensions);
        let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
        assert_resize_filters_succeed(fixture, output_dimensions, output_byte_len, selected_filter);
    }
}

fn bench_scale_comparison(
    criterion: &mut Criterion,
    fixture: &RgbaFixture,
    scale: Scale,
    comparison: &ResizeComparison,
) {
    let output_dimensions = scale.dimensions_for(fixture.dimensions);
    report_tiling_plan(
        scale,
        fixture.dimensions,
        output_dimensions,
        Some(comparison.filter_name.as_str()),
    );
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();

    let group_name = format!(
        "resize_filters/{}/{}-{}x{}",
        fixture.name,
        scale.label,
        output_dimensions.width(),
        output_dimensions.height()
    );
    let mut group = criterion.benchmark_group(group_name);

    group.sampling_mode(SamplingMode::Flat);
    group.throughput(Throughput::Bytes(output_byte_len as u64));
    bench_comparison_filter(
        &mut group,
        fixture,
        output_dimensions,
        output_byte_len,
        comparison,
    );
    group.finish();
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

    let group_name = format!(
        "resize_filters/{}/{}-{}x{}",
        fixture.name,
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
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    selected_filter: Option<&str>,
) {
    let plan = match selected_filter {
        Some("area" | "box") => {
            resize_rgba_area_dynamic_tiling_plan(source_dimensions, output_dimensions)
                .expect("area dynamic tiling plan should fit benchmark dimensions")
        }
        Some("nearest") => {
            resize_rgba_nearest_dynamic_tiling_plan(source_dimensions, output_dimensions)
                .expect("nearest dynamic tiling plan should fit benchmark dimensions")
        }
        Some("bilinear") => {
            resize_rgba_bilinear_dynamic_tiling_plan(source_dimensions, output_dimensions)
                .expect("bilinear dynamic tiling plan should fit benchmark dimensions")
        }
        _ => Some(plan_row_bands(
            output_dimensions.width() as usize,
            output_dimensions.height() as usize,
            DEFAULT_ROW_BAND_TILING,
        )),
    };
    let enabled = plan.is_some_and(|plan| plan.band_count > 1);
    let plan = plan.unwrap_or_else(|| {
        plan_row_bands(
            output_dimensions.width() as usize,
            output_dimensions.height() as usize,
            DEFAULT_ROW_BAND_TILING,
        )
    });

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

fn bench_comparison_filter(
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    comparison: &ResizeComparison,
) {
    match comparison.operation {
        CompareOperation::Resize(resize_into) => {
            group.bench_function(comparison.benchmark_id.as_str(), |bencher| {
                let mut output_rgba = vec![0; output_byte_len];

                bencher.iter(|| {
                    resize_into(
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
        CompareOperation::NearestAntialias(filter) => {
            group.bench_function(comparison.benchmark_id.as_str(), |bencher| {
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
        CompareOperation::Antialias(antialias_into) => {
            group.bench_function(comparison.benchmark_id.as_str(), |bencher| {
                let resized_rgba =
                    resized_nearest_fixture(fixture, output_dimensions, output_byte_len);
                let mut output_rgba = vec![0; output_byte_len];

                bencher.iter(|| {
                    antialias_into(
                        black_box(&resized_rgba),
                        output_dimensions,
                        black_box(&mut output_rgba),
                    )
                    .unwrap();
                    black_box(&output_rgba);
                });
            });
        }
    }
}

fn assert_comparison_succeeds(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    comparison: &ResizeComparison,
) {
    match comparison.operation {
        CompareOperation::Resize(resize_into) => {
            let mut output_rgba = vec![0xA5; output_byte_len];
            resize_into(
                &fixture.rgba,
                fixture.dimensions,
                output_dimensions,
                &mut output_rgba,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{} {} resize should succeed: {error}",
                    comparison.filter_name, comparison.implementation
                )
            });
        }
        CompareOperation::NearestAntialias(filter) => {
            nearest_antialias_for_check(fixture, output_dimensions, output_byte_len, filter)
                .unwrap_or_else(|error| {
                    panic!(
                        "{} {} resize should succeed: {error}",
                        comparison.filter_name, comparison.implementation
                    )
                });
        }
        CompareOperation::Antialias(antialias_into) => {
            let resized_rgba = resized_nearest_fixture(fixture, output_dimensions, output_byte_len);
            let mut output_rgba = vec![0xA5; output_byte_len];
            antialias_into(&resized_rgba, output_dimensions, &mut output_rgba).unwrap_or_else(
                |error| {
                    panic!(
                        "{} {} should succeed: {error}",
                        comparison.filter_name, comparison.implementation
                    )
                },
            );
        }
    }
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
        panic!(
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

fn requested_resize_comparison() -> Option<ResizeComparison> {
    let benchmark_id = env::var("RESIZE_FILTER_COMPARE_ID").ok()?;
    if benchmark_id.is_empty() {
        return None;
    }

    let filter_name = env::var("RESIZE_FILTER")
        .unwrap_or_else(|_| panic!("RESIZE_FILTER is required in comparison mode"));
    let implementation = env::var("RESIZE_FILTER_IMPLEMENTATION")
        .unwrap_or_else(|_| panic!("RESIZE_FILTER_IMPLEMENTATION is required in comparison mode"));
    let operation = compare_operation_for(&filter_name, &implementation).unwrap_or_else(|| {
        panic!("unsupported resize comparison implementation `{filter_name}:{implementation}`")
    });

    Some(ResizeComparison {
        benchmark_id,
        filter_name,
        implementation,
        operation,
    })
}

fn compare_operation_for(filter_name: &str, implementation: &str) -> Option<CompareOperation> {
    match (filter_name, implementation) {
        ("nearest", "reference") => {
            Some(CompareOperation::Resize(resize_rgba_nearest_reference_into))
        }
        ("nearest", "scalar") => Some(CompareOperation::Resize(resize_rgba_nearest_scalar_into)),
        ("nearest", "tiling") => nearest_tiling_compare_operation(),
        ("nearest_aa", "reference") => Some(CompareOperation::NearestAntialias(
            NearestAntialiasFilter::new(
                "nearest_aa_reference",
                resize_rgba_nearest_reference_into,
                antialias_rgba_box3_reference_into,
            ),
        )),
        ("nearest_aa", "scalar") => Some(CompareOperation::NearestAntialias(
            NearestAntialiasFilter::new(
                "nearest_aa",
                resize_rgba_nearest_scalar_into,
                antialias_rgba_box3_into,
            ),
        )),
        ("nearest_aa", "tiling") => nearest_antialias_tiling_compare_operation(),
        ("bilinear", "reference") => Some(CompareOperation::Resize(
            resize_rgba_bilinear_reference_into,
        )),
        ("bilinear", "scalar") => Some(CompareOperation::Resize(resize_rgba_bilinear_scalar_into)),
        ("bilinear", "scalar_2") => Some(CompareOperation::Resize(resize_rgba_bilinear_2_into)),
        ("bilinear", "tiling") => bilinear_tiling_compare_operation(),
        ("trilinear", "reference") => Some(CompareOperation::Resize(
            resize_rgba_trilinear_reference_into,
        )),
        ("trilinear", "scalar") => Some(CompareOperation::Resize(resize_rgba_trilinear_into)),
        ("trilinear", "tiling") => planned_tiling_compare_operation("trilinear"),
        ("bicubic", "reference") => {
            Some(CompareOperation::Resize(resize_rgba_bicubic_reference_into))
        }
        ("bicubic", "scalar") => Some(CompareOperation::Resize(resize_rgba_bicubic_into)),
        ("bicubic", "scalar_2") => Some(CompareOperation::Resize(resize_rgba_bicubic_2_into)),
        ("bicubic_fixed", "scalar_2") => {
            Some(CompareOperation::Resize(resize_rgba_bicubic_2_fixed_into))
        }
        ("bicubic_scale_aware", "scalar_2") => Some(CompareOperation::Resize(
            resize_rgba_bicubic_2_scale_aware_into,
        )),
        ("bicubic", "tiling") => planned_tiling_compare_operation("bicubic"),
        ("lanczos2", "reference") => Some(CompareOperation::Resize(
            resize_rgba_lanczos2_reference_into,
        )),
        ("lanczos2", "scalar") => Some(CompareOperation::Resize(resize_rgba_lanczos2_into)),
        ("lanczos2", "tiling") => planned_tiling_compare_operation("lanczos2"),
        ("lanczos2_scale_aware", "reference") => Some(CompareOperation::Resize(
            resize_rgba_lanczos2_scale_aware_reference_into,
        )),
        ("lanczos2_scale_aware", "scalar") => Some(CompareOperation::Resize(
            resize_rgba_lanczos2_scale_aware_into,
        )),
        ("lanczos2_scale_aware", "tiling") => {
            planned_tiling_compare_operation("lanczos2_scale_aware")
        }
        ("lanczos3", "reference") => Some(CompareOperation::Resize(
            resize_rgba_lanczos3_reference_into,
        )),
        ("lanczos3", "scalar") => Some(CompareOperation::Resize(resize_rgba_lanczos3_into)),
        ("lanczos3", "scalar_2") => Some(CompareOperation::Resize(resize_rgba_lanczos3_2_into)),
        ("lanczos3_fixed", "scalar_2") => {
            Some(CompareOperation::Resize(resize_rgba_lanczos3_2_fixed_into))
        }
        ("lanczos3_scale_aware", "scalar_2") => Some(CompareOperation::Resize(
            resize_rgba_lanczos3_2_scale_aware_into,
        )),
        ("lanczos3", "tiling") => planned_tiling_compare_operation("lanczos3"),
        ("lanczos3_scale_aware", "reference") => Some(CompareOperation::Resize(
            resize_rgba_lanczos3_scale_aware_reference_into,
        )),
        ("lanczos3_scale_aware", "scalar") => Some(CompareOperation::Resize(
            resize_rgba_lanczos3_scale_aware_into,
        )),
        ("lanczos3_scale_aware", "tiling") => {
            planned_tiling_compare_operation("lanczos3_scale_aware")
        }
        ("area", "reference") => Some(CompareOperation::Resize(resize_rgba_area_reference_into)),
        ("area", "scalar") => Some(CompareOperation::Resize(resize_rgba_area_scalar_into)),
        ("area", "tiling") => area_tiling_compare_operation(),
        ("box", "reference") => Some(CompareOperation::Resize(resize_rgba_area_reference_into)),
        ("box", "scalar") => Some(CompareOperation::Resize(resize_rgba_area_scalar_into)),
        ("box", "tiling") => area_tiling_compare_operation(),
        ("antialias", "reference") => Some(CompareOperation::Antialias(
            antialias_rgba_box3_reference_into,
        )),
        ("antialias", "scalar") => Some(CompareOperation::Antialias(antialias_rgba_box3_into)),
        ("antialias", "tiling") => planned_tiling_compare_operation("antialias"),
        _ => None,
    }
}

fn planned_tiling_compare_operation(filter_name: &str) -> Option<CompareOperation> {
    panic!("resize:{filter_name}:tiling is planned but not implemented yet")
}

#[cfg(feature = "tiling")]
fn nearest_tiling_compare_operation() -> Option<CompareOperation> {
    Some(CompareOperation::Resize(resize_rgba_nearest_tiling_into))
}

#[cfg(not(feature = "tiling"))]
fn nearest_tiling_compare_operation() -> Option<CompareOperation> {
    None
}

#[cfg(feature = "tiling")]
fn nearest_antialias_tiling_compare_operation() -> Option<CompareOperation> {
    Some(CompareOperation::NearestAntialias(
        NearestAntialiasFilter::new(
            "nearest_aa_tiling",
            resize_rgba_nearest_tiling_into,
            antialias_rgba_box3_into,
        ),
    ))
}

#[cfg(not(feature = "tiling"))]
fn nearest_antialias_tiling_compare_operation() -> Option<CompareOperation> {
    None
}

#[cfg(feature = "tiling")]
fn area_tiling_compare_operation() -> Option<CompareOperation> {
    Some(CompareOperation::Resize(resize_rgba_area_tiling_into))
}

#[cfg(not(feature = "tiling"))]
fn area_tiling_compare_operation() -> Option<CompareOperation> {
    None
}

#[cfg(feature = "tiling")]
fn bilinear_tiling_compare_operation() -> Option<CompareOperation> {
    Some(CompareOperation::Resize(resize_rgba_bilinear_tiling_into))
}

#[cfg(not(feature = "tiling"))]
fn bilinear_tiling_compare_operation() -> Option<CompareOperation> {
    None
}

fn requested_resize_filter() -> Option<String> {
    let filter = env::var("RESIZE_FILTER").ok()?;
    if filter.is_empty() {
        return None;
    }

    for selected in filter.split(',') {
        if selected == "antialias"
            || NEAREST_ANTIALIAS_FILTERS
                .iter()
                .any(|candidate| base_filter_name(candidate.name) == selected)
            || RESIZE_FILTERS
                .iter()
                .any(|candidate| base_filter_name(candidate.name) == selected)
            || IMAGE_RESIZE_FILTERS
                .iter()
                .any(|candidate| base_filter_name(candidate.name) == selected)
        {
            continue;
        }

        panic!("unknown resize filter `{selected}`");
    }

    Some(filter)
}

fn should_measure_filter(filter_name: &str, selected_filter: Option<&str>) -> bool {
    filter_matches_selection(filter_name, selected_filter) && !filter_name.ends_with("_reference")
}

fn filter_matches_selection(filter_name: &str, selected_filter: Option<&str>) -> bool {
    selected_filter.is_none_or(|selected| {
        selected
            .split(',')
            .any(|selected| base_filter_name(filter_name) == selected)
    })
}

fn should_bench_antialias(selected_filter: Option<&str>) -> bool {
    selected_filter
        .is_none_or(|selected| selected.split(',').any(|selected| selected == "antialias"))
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

fn load_benchmark_fixtures() -> Vec<RgbaFixture> {
    let fixture_paths = benchmark_fixture_paths();
    fixture_paths
        .iter()
        .map(|fixture_path| load_fixture(fixture_path))
        .collect()
}

fn benchmark_fixture_paths() -> Vec<PathBuf> {
    let fixture_dir = benchmark_fixture_dir();
    if let Ok(fixture_name) = env::var("RESIZE_FIXTURE") {
        return vec![fixture_dir.join(fixture_name)];
    }
    if let Ok(fixture_names) = env::var("RESIZE_FIXTURES") {
        return fixture_names
            .split(',')
            .filter(|name| !name.is_empty())
            .map(|name| fixture_dir.join(name))
            .collect();
    }

    let mut fixture_paths: Vec<PathBuf> = fs::read_dir(&fixture_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", fixture_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| {
                    panic!("failed to read {} entry: {error}", fixture_dir.display())
                })
                .path()
        })
        .filter(|path| is_supported_fixture_image(path))
        .collect();

    fixture_paths.sort();
    assert!(
        !fixture_paths.is_empty(),
        "{} should contain at least one PNG benchmark fixture",
        fixture_dir.display()
    );
    fixture_paths
}

fn is_supported_fixture_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("png")
                || extension.eq_ignore_ascii_case("jpg")
                || extension.eq_ignore_ascii_case("jpeg")
        })
}

fn load_fixture(fixture_path: &Path) -> RgbaFixture {
    let image = ImageReader::open(fixture_path)
        .unwrap_or_else(|error| panic!("failed to open {}: {error}", fixture_path.display()))
        .decode()
        .unwrap_or_else(|error| panic!("failed to decode {}: {error}", fixture_path.display()))
        .to_rgba8();

    let dimensions = ImageDimensions::new(image.width(), image.height()).unwrap();

    let rgba = image.into_raw();
    let image = ImageBuffer::from_raw(dimensions.width(), dimensions.height(), rgba.clone())
        .expect("decoded RGBA fixture should match its dimensions");

    RgbaFixture {
        name: fixture_benchmark_name(fixture_path),
        dimensions,
        image,
        rgba,
    }
}

fn benchmark_fixture_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .map(|repo_root| repo_root.join("benchmark-fixtures"))
        .expect("crate should live under crates/ditherette-wasm")
}

fn fixture_benchmark_name(fixture_path: &Path) -> String {
    let stem = fixture_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("fixture");
    let mut name = String::with_capacity(stem.len());
    for character in stem.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_lowercase());
        } else {
            name.push('_');
        }
    }
    while name.contains("__") {
        name = name.replace("__", "_");
    }
    name.trim_matches('_').to_owned()
}

#[derive(Debug)]
struct ResizeComparison {
    benchmark_id: String,
    filter_name: String,
    implementation: String,
    operation: CompareOperation,
}

#[derive(Debug, Clone, Copy)]
enum CompareOperation {
    Resize(ResizeInto),
    NearestAntialias(NearestAntialiasFilter),
    Antialias(AntialiasInto),
}

#[derive(Debug)]
struct RgbaFixture {
    name: String,
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
