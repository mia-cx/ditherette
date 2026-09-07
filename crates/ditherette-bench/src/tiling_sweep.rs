//! Empirical resize row-band tiling sweep.
//!
//! This command measures real production resize subjects across the same image
//! fixtures used by scalar resize optimization, records actual dimensions, and
//! emits smooth curve coefficients instead of hand-authored size bands.

use std::{
    fs::{self, File},
    hint::black_box,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use ditherette_bench_api::{
    ResizeBenchSubject, ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams,
};
use serde::Serialize;

use crate::{
    cli::{csv_set, Flags},
    error::BenchError,
    fixture::fixtures_from_flags,
    registry::Registry,
    result::{verify_with_bounds, SampleStats, VerificationBounds},
    util::{checksum, format_ns, output_dimensions, RGBA_CHANNELS},
};

const DEFAULT_OUTPUT_DIR: &str = "crates/ditherette-bench/target/tiling-sweep";
const DEFAULT_FILTERS: [&str; 9] = [
    "nearest",
    "area",
    "bilinear",
    "bicubic",
    "bicubic-scale-aware",
    "lanczos2",
    "lanczos2-scale-aware",
    "lanczos3",
    "lanczos3-scale-aware",
];
const DEFAULT_FIXTURES: &str = "Celeste_Insta_selfie,Celeste_box_art";
const DEFAULT_SAMPLES: usize = 50;
const DEFAULT_TARGET_SAMPLE_TIME: Duration = Duration::from_millis(1);
const DEFAULT_WARMUP: usize = 30;
const DEFAULT_MIN_BAND_HEIGHT: u32 = 32;
const MAX_DIMENSION_CASES_PER_FIXTURE: usize = 26;
const BALANCED_CHUNKS_PER_WORKER: [u32; 6] = [1, 2, 3, 4, 6, 8];
const EPSILON: f64 = 0.000_001;

pub(crate) fn tiling_sweep_command(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let flags = Flags::parse(args)?;
    let options = SweepOptions::from_flags(&flags)?;
    let fixtures = fixtures_from_sweep_flags(&flags)?;
    let subjects = subjects_for_filters(registry, &options.filters)?;
    let dimensions = dimensions_for_fixtures(&fixtures, options.max_cases_per_fixture);

    fs::create_dir_all(&options.output_dir).map_err(BenchError::io)?;
    let raw_path = options.output_dir.join("raw.csv");
    let best_path = options.output_dir.join("best.csv");
    let curves_path = options.output_dir.join("curves.json");
    let report_path = options.output_dir.join("report.md");

    let mut rows = Vec::new();
    let total_cases = subjects.len()
        * dimensions
            .iter()
            .flatten()
            .map(|output| {
                tiling_candidates(
                    output.1,
                    &options.band_heights,
                    &options.worker_counts,
                    options.min_band_height,
                )
                .len()
            })
            .sum::<usize>();
    eprintln!(
        "tiling sweep: fixtures={} filters={} dimension_sets={} band_heights={} worker_counts={} cases={} samples={} target_sample={} warmup={}",
        fixtures.len(),
        subjects.len(),
        dimensions.iter().map(Vec::len).sum::<usize>(),
        options.band_heights.len(),
        options.worker_counts.len(),
        total_cases,
        options.samples,
        format_duration(options.target_sample_time),
        options.warmup_iterations,
    );

    let mut case_index = 0usize;
    for (fixture_index, fixture) in fixtures.iter().enumerate() {
        for subject in &subjects {
            for output in &dimensions[fixture_index] {
                let scalar = measure_subject(
                    subject,
                    &fixture.rgba,
                    (fixture.width, fixture.height),
                    *output,
                    options.samples,
                    options.target_sample_time,
                    options.warmup_iterations,
                )?;

                let worker_budget = ditherette_wasm::prod::tiling::WorkerBudget::new(
                    options.worker_counts.iter().copied().max().unwrap_or(1),
                );
                for (band_height, worker_count) in tiling_candidates(
                    output.1,
                    &options.band_heights,
                    &options.worker_counts,
                    options.min_band_height,
                ) {
                    let actual_band_height = effective_band_height(band_height, output.1);
                    let band_count = output.1.div_ceil(actual_band_height);
                    case_index += 1;
                    let tiled = measure_subject_row_bands(
                        subject,
                        &fixture.rgba,
                        (fixture.width, fixture.height),
                        *output,
                        band_height,
                        worker_count,
                        options.samples,
                        options.target_sample_time,
                        options.warmup_iterations,
                    )?;
                    let verification = verify_with_bounds(
                        &scalar.output,
                        &tiled.output,
                        bounds_for_subject(subject),
                    );
                    if !verification.passed {
                        return Err(BenchError::Runtime(format!(
                            "tiled row-range output failed verification for {} {} {}x{} band_height={} workers={}: {:?}",
                            subject.descriptor.id,
                            fixture.id,
                            output.0,
                            output.1,
                            band_height,
                            worker_count,
                            verification.first_mismatch,
                        )));
                    }
                    let effective_worker_count =
                        worker_budget.active_workers(worker_count, band_count);
                    let pixels_per_band =
                        u64::from(output.0) * u64::from(actual_band_height.min(output.1));
                    let speedup = scalar.stats.median_ns / tiled.stats.median_ns;
                    let worker_efficiency = speedup / f64::from(effective_worker_count);
                    rows.push(SweepRow {
                        fixture: fixture.id.clone(),
                        subject: subject.descriptor.id.to_string(),
                        filter: filter_label(subject),
                        source_width: fixture.width,
                        source_height: fixture.height,
                        output_width: output.0,
                        output_height: output.1,
                        output_pixels: u64::from(output.0) * u64::from(output.1),
                        scale_x: f64::from(output.0) / f64::from(fixture.width),
                        scale_y: f64::from(output.1) / f64::from(fixture.height),
                        band_height,
                        band_count,
                        worker_count,
                        effective_worker_count,
                        worker_efficiency,
                        pixels_per_band,
                        scalar_median_ns: scalar.stats.median_ns,
                        tiled_median_ns: tiled.stats.median_ns,
                        speedup,
                        tiled_p95_ns: tiled.stats.p95_ns,
                        tiled_stdev_ns: tiled.stats.stdev_ns,
                        checksum: checksum(&tiled.output),
                    });

                    eprintln!(
                        "{case_index}/{total_cases} {} {} {}x{} band={} workers={} speedup={:.3} eff={:.3} tiled={}",
                        subject.descriptor.id,
                        fixture.id,
                        output.0,
                        output.1,
                        band_height,
                        worker_count,
                        speedup,
                        worker_efficiency,
                        format_ns(tiled.stats.median_ns)
                    );
                }
            }
        }
    }

    write_raw_csv(&raw_path, &rows)?;
    let best_rows = best_rows(&rows);
    write_best_csv(&best_path, &best_rows)?;
    let curves = fit_curves(&rows);
    write_json(&curves_path, &curves)?;
    write_report(&report_path, &rows, &best_rows, &curves)?;

    println!("wrote {}", raw_path.display());
    println!("wrote {}", best_path.display());
    println!("wrote {}", curves_path.display());
    println!("wrote {}", report_path.display());
    Ok(())
}

struct SweepOptions {
    output_dir: PathBuf,
    filters: Vec<String>,
    band_heights: Vec<u32>,
    worker_counts: Vec<u32>,
    min_band_height: u32,
    samples: usize,
    target_sample_time: Duration,
    warmup_iterations: usize,
    max_cases_per_fixture: usize,
}

impl SweepOptions {
    fn from_flags(flags: &Flags) -> Result<Self, BenchError> {
        Ok(Self {
            output_dir: flags
                .optional("--output-dir")
                .unwrap_or(DEFAULT_OUTPUT_DIR)
                .into(),
            filters: flags
                .optional("--filters")
                .map(csv_set)
                .unwrap_or_else(|| DEFAULT_FILTERS.iter().map(ToString::to_string).collect()),
            band_heights: flags
                .optional("--band-heights")
                .map(parse_band_heights)
                .transpose()?
                .unwrap_or_else(default_band_heights),
            worker_counts: flags
                .optional("--worker-counts")
                .map(parse_worker_counts)
                .transpose()?
                .unwrap_or_else(default_worker_counts),
            min_band_height: flags
                .optional("--min-band-height")
                .map(parse_u32)
                .transpose()?
                .unwrap_or(DEFAULT_MIN_BAND_HEIGHT),
            samples: flags
                .optional("--sample-size")
                .or_else(|| flags.optional("--samples"))
                .map(parse_usize)
                .transpose()?
                .unwrap_or(DEFAULT_SAMPLES),
            target_sample_time: flags
                .optional("--target-sample-time")
                .or_else(|| flags.optional("--target-sample-ms"))
                .map(parse_duration_millis)
                .transpose()?
                .unwrap_or(DEFAULT_TARGET_SAMPLE_TIME),
            warmup_iterations: flags
                .optional("--warm-up-iterations")
                .or_else(|| flags.optional("--warmup-iterations"))
                .map(parse_usize)
                .transpose()?
                .unwrap_or(DEFAULT_WARMUP),
            max_cases_per_fixture: flags
                .optional("--max-cases-per-fixture")
                .map(parse_usize)
                .transpose()?
                .unwrap_or(MAX_DIMENSION_CASES_PER_FIXTURE),
        })
    }
}

fn fixtures_from_sweep_flags(flags: &Flags) -> Result<Vec<crate::fixture::Fixture>, BenchError> {
    let fixture_args = if flags.optional("--fixtures").is_some() {
        Vec::new()
    } else {
        vec!["--fixtures".to_owned(), DEFAULT_FIXTURES.to_owned()]
    };
    if fixture_args.is_empty() {
        fixtures_from_flags(flags)
    } else {
        let flags = Flags::parse(&fixture_args)?;
        fixtures_from_flags(&flags)
    }
}

fn subjects_for_filters(
    registry: &Registry,
    filters: &[String],
) -> Result<Vec<ResizeBenchSubject>, BenchError> {
    filters
        .iter()
        .map(|filter| {
            let subject = registry.resize_subject(subject_for_filter(filter))?;
            if subject.descriptor.id.filter() == "nearest"
                && ![
                    "prod:resize:nearest:scalar",
                    "candidate:resize:nearest:legacy",
                ]
                .contains(&subject.descriptor.id.as_str())
            {
                return Err(BenchError::Config(
                    "Nearest row-band sweeps require the landed production nearest implementation."
                        .to_owned(),
                ));
            }
            Ok(subject)
        })
        .collect()
}

fn subject_for_filter(filter: &str) -> &str {
    match filter {
        "nearest" => "prod:resize:nearest:scalar",
        "area" => "prod:resize:area:scalar",
        "bilinear" => "prod:resize:bilinear:scalar",
        "bicubic" => "prod:resize:bicubic:catmull-rom",
        "bicubic-scale-aware" => "prod:resize:bicubic:catmull-rom-scale-aware",
        "lanczos2" => "prod:resize:lanczos2:fixed",
        "lanczos2-scale-aware" => "prod:resize:lanczos2:scale-aware",
        "lanczos3" => "prod:resize:lanczos3:fixed",
        "lanczos3-scale-aware" => "prod:resize:lanczos3:scale-aware",
        other => other,
    }
}

#[cfg(test)]
mod nearest_identity_tests {
    use super::*;

    #[test]
    fn landed_row_band_sweep_defaults_to_production_and_accepts_its_historical_alias() {
        let registry = Registry::load();
        let selected = subjects_for_filters(&registry, &["nearest".into()]).unwrap();
        assert_eq!(
            selected[0].descriptor.id.as_str(),
            "prod:resize:nearest:scalar"
        );
        for id in [
            "prod:resize:nearest:scalar",
            "candidate:resize:nearest:legacy",
        ] {
            let selected = subjects_for_filters(&registry, &[id.into()]).unwrap();
            assert!(selected[0]
                .descriptor
                .source_file
                .ends_with("prod/resize/scalar/nearest/mod.rs"));
        }
        for id in [
            "candidate:resize:nearest:incremental",
            "spec:resize:nearest:scalar",
        ] {
            assert!(
                subjects_for_filters(&registry, &[id.into()]).is_err(),
                "{id}"
            );
        }
    }
}

fn filter_label(subject: &ResizeBenchSubject) -> String {
    let id = &subject.descriptor.id;
    match (id.filter(), id.variant()) {
        ("bicubic", "catmull-rom-scale-aware") => "bicubic-scale-aware".to_owned(),
        ("lanczos2", "scale-aware") => "lanczos2-scale-aware".to_owned(),
        ("lanczos3", "scale-aware") => "lanczos3-scale-aware".to_owned(),
        (filter, _) => filter.to_owned(),
    }
}

fn dimensions_for_fixtures(
    fixtures: &[crate::fixture::Fixture],
    max_cases_per_fixture: usize,
) -> Vec<Vec<(u32, u32)>> {
    fixtures
        .iter()
        .map(|fixture| {
            let mut dimensions = Vec::new();
            for scale in dimension_scales() {
                dimensions.push(output_dimensions(
                    fixture.width,
                    fixture.height,
                    scale,
                    scale,
                ));
            }
            for long_edge in [
                64, 80, 100, 160, 200, 260, 325, 400, 520, 650, 800, 1042, 1300, 1950, 2340, 2470,
                2574,
            ] {
                let scale = f64::from(long_edge) / f64::from(fixture.width.max(fixture.height));
                dimensions.push(output_dimensions(
                    fixture.width,
                    fixture.height,
                    scale,
                    scale,
                ));
            }
            dimensions.sort_unstable();
            dimensions.dedup();
            dimensions
                .into_iter()
                .filter(|(width, height)| *width > 0 && *height > 0)
                .take(max_cases_per_fixture)
                .collect()
        })
        .collect()
}

fn dimension_scales() -> Vec<f64> {
    vec![
        0.0625, 0.1, 0.125, 0.16, 0.2, 0.25, 0.33, 0.5, 0.75, 0.9, 0.95, 0.97, 0.98, 0.99, 1.0,
        1.01, 1.02, 1.03, 1.05, 1.07, 1.1, 1.25, 1.5, 2.0, 3.0, 4.0,
    ]
}

fn default_band_heights() -> Vec<u32> {
    vec![
        32,
        48,
        64,
        96,
        128,
        192,
        256,
        384,
        512,
        768,
        1024,
        1536,
        2048,
        u32::MAX,
    ]
}

fn default_worker_counts() -> Vec<u32> {
    let available = thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1);
    ditherette_wasm::prod::tiling::WorkerBudget::from_available_parallelism(available)
        .worker_counts()
        .collect()
}

fn tiling_candidates(
    output_height: u32,
    fixed_band_heights: &[u32],
    worker_counts: &[u32],
    min_band_height: u32,
) -> Vec<(u32, u32)> {
    let budget = ditherette_wasm::prod::tiling::WorkerBudget::new(
        worker_counts.iter().copied().max().unwrap_or(1),
    );
    let mut candidates = Vec::new();

    let min_band_height = min_band_height.max(1).min(output_height);

    for &band_height in fixed_band_heights {
        if effective_band_height(band_height, output_height) < min_band_height {
            continue;
        }
        let band_count = output_height.div_ceil(effective_band_height(band_height, output_height));
        candidates.extend(
            worker_counts
                .iter()
                .copied()
                .filter(|&worker_count| budget.can_use_workers(worker_count, band_count))
                .map(|worker_count| (band_height, worker_count)),
        );
    }

    for &worker_count in worker_counts {
        if !budget.can_use_workers(worker_count, output_height) {
            continue;
        }
        for chunks_per_worker in BALANCED_CHUNKS_PER_WORKER {
            let target_band_count = worker_count
                .saturating_mul(chunks_per_worker)
                .clamp(1, output_height);
            let band_height = output_height
                .div_ceil(target_band_count)
                .max(min_band_height);
            let band_count = output_height.div_ceil(band_height);
            if budget.can_use_workers(worker_count, band_count) {
                candidates.push((band_height, worker_count));
            }
        }
    }

    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

fn effective_band_height(band_height: u32, output_height: u32) -> u32 {
    if band_height == u32::MAX {
        output_height
    } else {
        band_height
    }
}

struct MeasuredOutput {
    output: Vec<u8>,
    stats: SampleStats,
}

fn measure_subject(
    subject: &ResizeBenchSubject,
    source: &[u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    samples: usize,
    target_sample_time: Duration,
    warmup_iterations: usize,
) -> Result<MeasuredOutput, BenchError> {
    let output = vec![0; output_len(output_dimensions)?];
    measure_output(
        samples,
        target_sample_time,
        warmup_iterations,
        output,
        |output| {
            run_subject(
                subject,
                black_box(source),
                black_box(output),
                source_dimensions,
                output_dimensions,
            )
        },
    )
}

fn measure_subject_row_bands(
    subject: &ResizeBenchSubject,
    source: &[u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    band_height: u32,
    worker_count: u32,
    samples: usize,
    target_sample_time: Duration,
    warmup_iterations: usize,
) -> Result<MeasuredOutput, BenchError> {
    let output = vec![0; output_len(output_dimensions)?];
    measure_output(
        samples,
        target_sample_time,
        warmup_iterations,
        output,
        |output| {
            run_subject_row_bands(
                subject,
                black_box(source),
                black_box(output),
                source_dimensions,
                output_dimensions,
                band_height,
                worker_count,
            )
        },
    )
}

fn measure_output(
    samples: usize,
    target_sample_time: Duration,
    warmup_iterations: usize,
    mut output: Vec<u8>,
    mut run: impl FnMut(&mut [u8]) -> Result<(), BenchError>,
) -> Result<MeasuredOutput, BenchError> {
    for _ in 0..warmup_iterations {
        run(&mut output)?;
        black_box(&output);
    }

    let iterations_per_sample = calibrate_iterations(target_sample_time, &mut output, &mut run)?;
    let mut sample_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        for _ in 0..iterations_per_sample {
            run(&mut output)?;
        }
        let elapsed = start.elapsed();
        black_box(&output);
        sample_ns.push(elapsed.as_nanos() as f64 / iterations_per_sample as f64);
    }
    Ok(MeasuredOutput {
        output,
        stats: SampleStats::from_samples(&sample_ns),
    })
}

fn calibrate_iterations(
    target_sample_time: Duration,
    output: &mut [u8],
    run: &mut impl FnMut(&mut [u8]) -> Result<(), BenchError>,
) -> Result<usize, BenchError> {
    let start = Instant::now();
    run(output)?;
    let elapsed = start.elapsed();
    black_box(&output);
    if elapsed >= target_sample_time || elapsed.is_zero() {
        return Ok(1);
    }

    let target_ns = target_sample_time.as_nanos().max(1);
    let elapsed_ns = elapsed.as_nanos().max(1);
    Ok(((target_ns + elapsed_ns / 2) / elapsed_ns).max(1) as usize)
}

fn run_subject(
    subject: &ResizeBenchSubject,
    source: &[u8],
    output: &mut [u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
) -> Result<(), BenchError> {
    (subject.resize_u8_rgba)(
        ResizeInputU8Rgba {
            data: source,
            width: source_dimensions.0,
            height: source_dimensions.1,
            row_stride_elements: usize::try_from(source_dimensions.0).unwrap() * RGBA_CHANNELS,
        },
        ResizeOutputU8Rgba {
            data: output,
            width: output_dimensions.0,
            height: output_dimensions.1,
            row_stride_elements: usize::try_from(output_dimensions.0).unwrap() * RGBA_CHANNELS,
        },
        &ResizeParams::default(),
    )
    .map_err(|error| BenchError::Runtime(format!("resize subject failed: {error}")))
}

fn run_subject_rows(
    subject: &ResizeBenchSubject,
    source: &[u8],
    output: &mut [u8],
    source_dimensions: (u32, u32),
    full_output_dimensions: (u32, u32),
    y_start: u32,
    band_output_dimensions: (u32, u32),
) -> Result<(), BenchError> {
    use ditherette_wasm::{
        image::{ImageView, ImageViewMut, Rgba8, RowStride},
        prod::resize::scalar::{
            area::resize_area_rgba8_rows_into,
            bicubic::resize_bicubic_rgba8_rows_into,
            bilinear::{
                alignment::ResizeAnchor as BilinearAnchor, resize_bilinear_rgba8_rows_into,
            },
            convolution::{ResizeAnchor as ConvolutionAnchor, SupportPolicy},
            lanczos::{resize_lanczos2_rgba8_rows_into, resize_lanczos3_rgba8_rows_into},
            nearest::{alignment::ResizeAnchor as NearestAnchor, resize_nearest_rgba8_rows_into},
        },
    };

    let source_view = ImageView::<Rgba8>::new(
        source,
        image_dimensions(source_dimensions)?,
        RowStride::new(usize::try_from(source_dimensions.0).unwrap() * RGBA_CHANNELS)
            .map_err(|error| BenchError::Runtime(error.to_string()))?,
    )
    .map_err(|error| BenchError::Runtime(error.to_string()))?;
    let output_view = ImageViewMut::<Rgba8>::new(
        output,
        image_dimensions(band_output_dimensions)?,
        RowStride::new(usize::try_from(band_output_dimensions.0).unwrap() * RGBA_CHANNELS)
            .map_err(|error| BenchError::Runtime(error.to_string()))?,
    )
    .map_err(|error| BenchError::Runtime(error.to_string()))?;
    let full_output_dimensions = image_dimensions(full_output_dimensions)?;

    match (
        subject.descriptor.id.filter(),
        subject.descriptor.id.variant(),
    ) {
        ("nearest", _) => resize_nearest_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            NearestAnchor::Center,
        ),
        ("area", _) => {
            resize_area_rgba8_rows_into(source_view, output_view, full_output_dimensions, y_start)
        }
        ("bilinear", _) => resize_bilinear_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            BilinearAnchor::Center,
        ),
        ("bicubic", "catmull-rom") => resize_bicubic_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            ConvolutionAnchor::Center,
            SupportPolicy::Fixed,
        ),
        ("bicubic", "catmull-rom-scale-aware") => resize_bicubic_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            ConvolutionAnchor::Center,
            SupportPolicy::ScaleAware,
        ),
        ("lanczos2", "fixed") => resize_lanczos2_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            ConvolutionAnchor::Center,
            SupportPolicy::Fixed,
        ),
        ("lanczos2", "scale-aware") => resize_lanczos2_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            ConvolutionAnchor::Center,
            SupportPolicy::ScaleAware,
        ),
        ("lanczos3", "fixed") => resize_lanczos3_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            ConvolutionAnchor::Center,
            SupportPolicy::Fixed,
        ),
        ("lanczos3", "scale-aware") => resize_lanczos3_rgba8_rows_into(
            source_view,
            output_view,
            full_output_dimensions,
            y_start,
            ConvolutionAnchor::Center,
            SupportPolicy::ScaleAware,
        ),
        _ => {
            return Err(BenchError::Config(format!(
                "tiling sweep has no row-range adapter for {}",
                subject.descriptor.id
            )))
        }
    }
    Ok(())
}

fn run_subject_row_bands(
    subject: &ResizeBenchSubject,
    source: &[u8],
    output: &mut [u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    band_height: u32,
    worker_count: u32,
) -> Result<(), BenchError> {
    if subject.descriptor.id.filter() == "nearest" {
        return run_nearest_subject_row_bands(
            source,
            output,
            source_dimensions,
            output_dimensions,
            band_height,
            worker_count,
        );
    }

    let actual_band_height = effective_band_height(band_height, output_dimensions.1);
    let Some(plan) = ditherette_wasm::prod::tiling::RowBandPlan::for_output_height(
        image_dimensions(output_dimensions)?,
        actual_band_height,
    ) else {
        return Err(BenchError::Config(format!(
            "invalid band height {band_height}"
        )));
    };

    let Some(work_plan) = ditherette_wasm::prod::tiling::RowBandWorkPlan::new(
        &plan,
        ditherette_wasm::prod::tiling::WorkerBudget::new(worker_count),
        worker_count,
    ) else {
        return Ok(());
    };
    let bands = plan
        .bands()
        .iter()
        .map(|band| (band.y_start(), band.y_end()))
        .collect::<Vec<_>>();
    let effective_worker_count = usize::try_from(work_plan.active_workers()).unwrap_or(usize::MAX);

    if effective_worker_count == 1 {
        for &(y_start, y_end) in &bands {
            let band_output = render_band(
                subject,
                source,
                source_dimensions,
                output_dimensions,
                y_start,
                y_end,
            )?;
            copy_band_output(&band_output, output, output_dimensions.0, y_start)?;
        }
        return Ok(());
    }

    let rendered_bands =
        thread::scope(|scope| {
            let handles = work_plan
                .assignments()
                .iter()
                .map(|assignment| {
                    let chunk = assignment
                        .bands()
                        .iter()
                        .map(|band| (band.y_start(), band.y_end()))
                        .collect::<Vec<_>>();
                    scope.spawn(move || -> Result<Vec<(u32, Vec<u8>)>, BenchError> {
                        chunk
                            .into_iter()
                            .map(|(y_start, y_end)| {
                                render_band(
                                    subject,
                                    source,
                                    source_dimensions,
                                    output_dimensions,
                                    y_start,
                                    y_end,
                                )
                                .map(|band_output| (y_start, band_output))
                            })
                            .collect()
                    })
                })
                .collect::<Vec<_>>();

            let mut rendered_bands = Vec::new();
            for handle in handles {
                rendered_bands.extend(handle.join().map_err(|_| {
                    BenchError::Runtime("tiling sweep worker panicked".to_owned())
                })??);
            }
            Ok::<_, BenchError>(rendered_bands)
        })?;

    for (y_start, band_output) in rendered_bands {
        copy_band_output(&band_output, output, output_dimensions.0, y_start)?;
    }
    Ok(())
}

fn render_band(
    subject: &ResizeBenchSubject,
    source: &[u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    y_start: u32,
    y_end: u32,
) -> Result<Vec<u8>, BenchError> {
    let band_output_dimensions = (output_dimensions.0, y_end - y_start);
    let mut band_output = vec![0; output_len(band_output_dimensions)?];
    run_subject_rows(
        subject,
        source,
        &mut band_output,
        source_dimensions,
        output_dimensions,
        y_start,
        band_output_dimensions,
    )?;
    Ok(band_output)
}

fn run_nearest_subject_row_bands(
    source: &[u8],
    output: &mut [u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    band_height: u32,
    worker_count: u32,
) -> Result<(), BenchError> {
    use ditherette_wasm::{
        image::{ImageView, ImageViewMut, Rgba8, RowStride},
        prod::resize::scalar::nearest::{
            alignment::ResizeAnchor, resize_nearest_rgba8_rows_with_plan_into, NearestResizePlan,
        },
    };

    let actual_band_height = effective_band_height(band_height, output_dimensions.1);
    let Some(plan) = ditherette_wasm::prod::tiling::RowBandPlan::for_output_height(
        image_dimensions(output_dimensions)?,
        actual_band_height,
    ) else {
        return Err(BenchError::Config(format!(
            "invalid band height {band_height}"
        )));
    };
    let Some(work_plan) = ditherette_wasm::prod::tiling::RowBandWorkPlan::new(
        &plan,
        ditherette_wasm::prod::tiling::WorkerBudget::new(worker_count),
        worker_count,
    ) else {
        return Ok(());
    };

    let source_dimensions = image_dimensions(source_dimensions)?;
    let output_dimensions = image_dimensions(output_dimensions)?;
    let resize_plan =
        NearestResizePlan::new(source_dimensions, output_dimensions, ResizeAnchor::Center);
    let source_stride = RowStride::new(source_dimensions.width_usize() * RGBA_CHANNELS)
        .map_err(|error| BenchError::Runtime(error.to_string()))?;

    let render_band = |y_start: u32, y_end: u32| -> Result<Vec<u8>, BenchError> {
        let band_dimensions = image_dimensions((output_dimensions.width(), y_end - y_start))?;
        let mut band_output = vec![0; output_len((output_dimensions.width(), y_end - y_start))?];
        let source_view = ImageView::<Rgba8>::new(source, source_dimensions, source_stride)
            .map_err(|error| BenchError::Runtime(error.to_string()))?;
        let output_stride = RowStride::new(output_dimensions.width_usize() * RGBA_CHANNELS)
            .map_err(|error| BenchError::Runtime(error.to_string()))?;
        let output_view =
            ImageViewMut::<Rgba8>::new(&mut band_output, band_dimensions, output_stride)
                .map_err(|error| BenchError::Runtime(error.to_string()))?;
        resize_nearest_rgba8_rows_with_plan_into(source_view, output_view, &resize_plan, y_start);
        Ok(band_output)
    };

    if work_plan.active_workers() == 1 {
        for band in plan.bands() {
            let band_output = render_band(band.y_start(), band.y_end())?;
            copy_band_output(
                &band_output,
                output,
                output_dimensions.width(),
                band.y_start(),
            )?;
        }
        return Ok(());
    }

    let rendered_bands = thread::scope(|scope| {
        let handles = work_plan
            .assignments()
            .iter()
            .map(|assignment| {
                let bands = assignment.bands().to_vec();
                let render_band = &render_band;
                scope.spawn(move || -> Result<Vec<(u32, Vec<u8>)>, BenchError> {
                    bands
                        .into_iter()
                        .map(|band| {
                            render_band(band.y_start(), band.y_end())
                                .map(|band_output| (band.y_start(), band_output))
                        })
                        .collect()
                })
            })
            .collect::<Vec<_>>();

        let mut rendered_bands = Vec::new();
        for handle in handles {
            rendered_bands.extend(handle.join().map_err(|_| {
                BenchError::Runtime("nearest tiling sweep worker panicked".to_owned())
            })??);
        }
        Ok::<_, BenchError>(rendered_bands)
    })?;

    for (y_start, band_output) in rendered_bands {
        copy_band_output(&band_output, output, output_dimensions.width(), y_start)?;
    }
    Ok(())
}

fn copy_band_output(
    band_output: &[u8],
    output: &mut [u8],
    output_width: u32,
    y_start: u32,
) -> Result<(), BenchError> {
    let row_len = usize::try_from(output_width).unwrap() * RGBA_CHANNELS;
    let output_start = usize::try_from(y_start).unwrap() * row_len;
    let output_end = output_start + band_output.len();
    output
        .get_mut(output_start..output_end)
        .ok_or_else(|| BenchError::Runtime("band output slice out of bounds".to_owned()))?
        .copy_from_slice(band_output);
    Ok(())
}

fn bounds_for_subject(subject: &ResizeBenchSubject) -> VerificationBounds {
    if subject.descriptor.id.filter() == "nearest" {
        VerificationBounds::exact()
    } else {
        VerificationBounds::bounded_default()
    }
}

#[derive(Clone)]
struct SweepRow {
    fixture: String,
    subject: String,
    filter: String,
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    output_pixels: u64,
    scale_x: f64,
    scale_y: f64,
    band_height: u32,
    band_count: u32,
    worker_count: u32,
    effective_worker_count: u32,
    worker_efficiency: f64,
    pixels_per_band: u64,
    scalar_median_ns: f64,
    tiled_median_ns: f64,
    speedup: f64,
    tiled_p95_ns: f64,
    tiled_stdev_ns: f64,
    checksum: String,
}

fn write_raw_csv(path: &Path, rows: &[SweepRow]) -> Result<(), BenchError> {
    let mut writer = csv_writer(path)?;
    writeln!(writer, "fixture,subject,filter,source_width,source_height,output_width,output_height,output_pixels,scale_x,scale_y,band_height,band_count,worker_count,effective_worker_count,worker_efficiency,pixels_per_band,scalar_median_ns,tiled_median_ns,speedup,tiled_p95_ns,tiled_stdev_ns,checksum").map_err(BenchError::io)?;
    for row in rows {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{},{},{:.6},{},{:.3},{:.3},{:.6},{:.3},{:.3},{}",
            row.fixture,
            row.subject,
            row.filter,
            row.source_width,
            row.source_height,
            row.output_width,
            row.output_height,
            row.output_pixels,
            row.scale_x,
            row.scale_y,
            row.band_height,
            row.band_count,
            row.worker_count,
            row.effective_worker_count,
            row.worker_efficiency,
            row.pixels_per_band,
            row.scalar_median_ns,
            row.tiled_median_ns,
            row.speedup,
            row.tiled_p95_ns,
            row.tiled_stdev_ns,
            row.checksum
        )
        .map_err(BenchError::io)?;
    }
    Ok(())
}

fn write_best_csv(path: &Path, rows: &[SweepRow]) -> Result<(), BenchError> {
    let mut writer = csv_writer(path)?;
    writeln!(writer, "fixture,subject,filter,source_width,source_height,output_width,output_height,output_pixels,best_band_height,best_band_count,best_worker_count,best_effective_worker_count,best_worker_efficiency,best_pixels_per_band,scalar_median_ns,best_tiled_median_ns,best_speedup").map_err(BenchError::io)?;
    for row in rows {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{},{:.3},{:.3},{:.6}",
            row.fixture,
            row.subject,
            row.filter,
            row.source_width,
            row.source_height,
            row.output_width,
            row.output_height,
            row.output_pixels,
            row.band_height,
            row.band_count,
            row.worker_count,
            row.effective_worker_count,
            row.worker_efficiency,
            row.pixels_per_band,
            row.scalar_median_ns,
            row.tiled_median_ns,
            row.speedup
        )
        .map_err(BenchError::io)?;
    }
    Ok(())
}

fn best_rows(rows: &[SweepRow]) -> Vec<SweepRow> {
    let mut best = Vec::new();
    for row in rows {
        if let Some(existing) = best.iter_mut().find(|existing: &&mut SweepRow| {
            existing.fixture == row.fixture
                && existing.subject == row.subject
                && existing.output_width == row.output_width
                && existing.output_height == row.output_height
        }) {
            if row.speedup > existing.speedup + EPSILON
                || ((row.speedup - existing.speedup).abs() <= EPSILON
                    && (row.worker_count, row.band_height)
                        < (existing.worker_count, existing.band_height))
            {
                *existing = row.clone();
            }
        } else {
            best.push(row.clone());
        }
    }
    best
}

#[derive(Serialize)]
struct CurveFile {
    schema: &'static str,
    description: &'static str,
    curves: Vec<Curve>,
}

#[derive(Serialize)]
struct Curve {
    filter: String,
    subject: String,
    samples: usize,
    band_formula: &'static str,
    worker_formula: &'static str,
    band_intercept: f64,
    band_log_output_pixels: f64,
    band_log_kernel_cost: f64,
    band_log_minify: f64,
    worker_intercept: f64,
    worker_log_output_pixels: f64,
    worker_log_kernel_cost: f64,
    worker_log_minify: f64,
    rmse_log_pixels_per_band: f64,
    rmse_log_worker_count: f64,
}

fn fit_curves(rows: &[SweepRow]) -> CurveFile {
    let best = best_rows(rows);
    let mut curves = Vec::new();
    let mut subjects = best
        .iter()
        .map(|row| row.subject.clone())
        .collect::<Vec<_>>();
    subjects.sort();
    subjects.dedup();

    for subject in subjects {
        let subject_rows = best
            .iter()
            .filter(|row| row.subject == subject && row.speedup > 1.0 + EPSILON)
            .collect::<Vec<_>>();
        if subject_rows.len() < 4 {
            continue;
        }
        let band_coefficients = fit_log_linear_target(&subject_rows, |row| {
            (row.pixels_per_band.max(1) as f64).ln()
        });
        let worker_coefficients = fit_log_linear_target(&subject_rows, |row| {
            f64::from(row.effective_worker_count.max(1)).ln()
        });
        let band_rmse = curve_rmse(&subject_rows, band_coefficients, |row| {
            (row.pixels_per_band.max(1) as f64).ln()
        });
        let worker_rmse = curve_rmse(&subject_rows, worker_coefficients, |row| {
            f64::from(row.effective_worker_count.max(1)).ln()
        });
        curves.push(Curve {
            filter: subject_rows[0].filter.clone(),
            subject,
            samples: subject_rows.len(),
            band_formula: "target_pixels_per_band = exp(intercept + a*ln(output_pixels) + b*ln(kernel_cost) + c*ln(max(source_w/output_w, source_h/output_h, 1)))",
            worker_formula: "target_worker_count = round(exp(intercept + a*ln(output_pixels) + b*ln(kernel_cost) + c*ln(max(source_w/output_w, source_h/output_h, 1))))",
            band_intercept: band_coefficients[0],
            band_log_output_pixels: band_coefficients[1],
            band_log_kernel_cost: band_coefficients[2],
            band_log_minify: band_coefficients[3],
            worker_intercept: worker_coefficients[0],
            worker_log_output_pixels: worker_coefficients[1],
            worker_log_kernel_cost: worker_coefficients[2],
            worker_log_minify: worker_coefficients[3],
            rmse_log_pixels_per_band: band_rmse,
            rmse_log_worker_count: worker_rmse,
        });
    }

    CurveFile {
        schema: "ditherette-tiling-sweep-curves-v1",
        description: "Smooth empirical row-band and worker-count tiling curves fitted from real fixture dimensions. Rows measure absolute-coordinate resize row-range adapters and verify tiled output against the scalar subject before recording timings.",
        curves,
    }
}

fn fit_log_linear_target(rows: &[&SweepRow], target: impl Fn(&SweepRow) -> f64) -> [f64; 4] {
    let lambda = 0.001;
    let mut xtx = [[0.0; 4]; 4];
    let mut xty = [0.0; 4];
    for row in rows {
        let x = curve_features(row);
        let y = target(row);
        for i in 0..4 {
            xty[i] += x[i] * y;
            for j in 0..4 {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    for (i, row) in xtx.iter_mut().enumerate() {
        row[i] += lambda;
    }
    solve_4x4(xtx, xty).unwrap_or([0.0, 1.0, 0.0, 0.0])
}

fn curve_rmse(
    rows: &[&SweepRow],
    coefficients: [f64; 4],
    target: impl Fn(&SweepRow) -> f64,
) -> f64 {
    let squared = rows
        .iter()
        .map(|row| {
            let predicted = dot(coefficients, curve_features(row));
            let actual = target(row);
            let error = predicted - actual;
            error * error
        })
        .sum::<f64>();
    (squared / rows.len() as f64).sqrt()
}

fn curve_features(row: &SweepRow) -> [f64; 4] {
    [
        1.0,
        (row.output_pixels.max(1) as f64).ln(),
        kernel_cost(&row.filter).ln(),
        minify(row).ln(),
    ]
}

fn kernel_cost(filter: &str) -> f64 {
    match filter {
        "nearest" => 1.0,
        "area" => 4.0,
        "bilinear" => 4.0,
        "bicubic" | "bicubic-scale-aware" | "lanczos2" | "lanczos2-scale-aware" => 16.0,
        "lanczos3" | "lanczos3-scale-aware" => 36.0,
        _ => 4.0,
    }
}

fn minify(row: &SweepRow) -> f64 {
    (f64::from(row.source_width) / f64::from(row.output_width))
        .max(f64::from(row.source_height) / f64::from(row.output_height))
        .max(1.0)
}

fn solve_4x4(mut a: [[f64; 4]; 4], mut b: [f64; 4]) -> Option<[f64; 4]> {
    for pivot in 0..4 {
        let mut max_row = pivot;
        for row in pivot + 1..4 {
            if a[row][pivot].abs() > a[max_row][pivot].abs() {
                max_row = row;
            }
        }
        if a[max_row][pivot].abs() < 1e-12 {
            return None;
        }
        a.swap(pivot, max_row);
        b.swap(pivot, max_row);
        let divisor = a[pivot][pivot];
        for column in pivot..4 {
            a[pivot][column] /= divisor;
        }
        b[pivot] /= divisor;
        for row in 0..4 {
            if row == pivot {
                continue;
            }
            let factor = a[row][pivot];
            for column in pivot..4 {
                a[row][column] -= factor * a[pivot][column];
            }
            b[row] -= factor * b[pivot];
        }
    }
    Some(b)
}

fn dot(left: [f64; 4], right: [f64; 4]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn write_json(path: &Path, curves: &CurveFile) -> Result<(), BenchError> {
    let json = serde_json::to_string_pretty(curves)
        .map_err(|error| BenchError::Runtime(error.to_string()))?;
    fs::write(path, json).map_err(BenchError::io)
}

fn write_report(
    path: &Path,
    rows: &[SweepRow],
    best_rows: &[SweepRow],
    curves: &CurveFile,
) -> Result<(), BenchError> {
    let mut writer = csv_writer(path)?;
    writeln!(writer, "# Tiling sweep report\n").map_err(BenchError::io)?;
    writeln!(writer, "rows: {}", rows.len()).map_err(BenchError::io)?;
    writeln!(writer, "\nNote: this command measures full-output tiled resize using absolute-coordinate row-range adapters. Each candidate writes every output row through `prod::tiling::RowBandPlan`/`RowBandWorkPlan` and verifies the reconstructed output against the scalar subject before recording timings.\n").map_err(BenchError::io)?;
    let tiled_win_rows = best_rows
        .iter()
        .filter(|row| row.speedup > 1.0 + EPSILON)
        .count();
    writeln!(writer, "best rows: {}", best_rows.len()).map_err(BenchError::io)?;
    writeln!(writer, "best rows with tiled speedup: {}", tiled_win_rows).map_err(BenchError::io)?;
    writeln!(writer, "curves: {}\n", curves.curves.len()).map_err(BenchError::io)?;
    writeln!(
        writer,
        "| filter | subject | samples | parameter | rmse | coefficients |"
    )
    .map_err(BenchError::io)?;
    writeln!(writer, "| --- | --- | ---: | --- | ---: | --- |").map_err(BenchError::io)?;
    for curve in &curves.curves {
        writeln!(
            writer,
            "| {} | `{}` | {} | pixels/band | {:.4} | intercept={:.4}, output={:.4}, kernel={:.4}, minify={:.4} |",
            curve.filter,
            curve.subject,
            curve.samples,
            curve.rmse_log_pixels_per_band,
            curve.band_intercept,
            curve.band_log_output_pixels,
            curve.band_log_kernel_cost,
            curve.band_log_minify,
        )
        .map_err(BenchError::io)?;
        writeln!(
            writer,
            "| {} | `{}` | {} | worker_count | {:.4} | intercept={:.4}, output={:.4}, kernel={:.4}, minify={:.4} |",
            curve.filter,
            curve.subject,
            curve.samples,
            curve.rmse_log_worker_count,
            curve.worker_intercept,
            curve.worker_log_output_pixels,
            curve.worker_log_kernel_cost,
            curve.worker_log_minify,
        )
        .map_err(BenchError::io)?;
    }
    Ok(())
}

fn csv_writer(path: &Path) -> Result<BufWriter<File>, BenchError> {
    Ok(BufWriter::new(File::create(path).map_err(BenchError::io)?))
}

fn parse_band_heights(value: &str) -> Result<Vec<u32>, BenchError> {
    value
        .split(',')
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value == "full" {
                return Ok(u32::MAX);
            }
            value.parse::<u32>().map_err(|error| {
                BenchError::Config(format!("invalid band height {value:?}: {error}"))
            })
        })
        .collect()
}

fn parse_duration_millis(value: &str) -> Result<Duration, BenchError> {
    let value = value.trim();
    if let Some(milliseconds) = value.strip_suffix("ms") {
        return parse_duration_seconds(milliseconds, 1_000.0);
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return parse_duration_seconds(seconds, 1.0);
    }
    parse_duration_seconds(value, 1_000.0)
}

fn parse_duration_seconds(value: &str, divisor: f64) -> Result<Duration, BenchError> {
    let value = value
        .parse::<f64>()
        .map_err(|error| BenchError::Config(format!("invalid duration {value:?}: {error}")))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(BenchError::Config(
            "duration must be greater than zero".to_owned(),
        ));
    }
    Ok(Duration::from_secs_f64(value / divisor))
}

fn format_duration(duration: Duration) -> String {
    if duration.as_millis() > 0 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{}ns", duration.as_nanos())
    }
}

fn parse_u32(value: &str) -> Result<u32, BenchError> {
    value
        .parse::<u32>()
        .map_err(|error| BenchError::Config(format!("invalid integer {value:?}: {error}")))
}

fn parse_worker_counts(value: &str) -> Result<Vec<u32>, BenchError> {
    value
        .split(',')
        .filter(|value| !value.is_empty())
        .map(|value| {
            let count = value.parse::<u32>().map_err(|error| {
                BenchError::Config(format!("invalid worker count {value:?}: {error}"))
            })?;
            if count == 0 {
                return Err(BenchError::Config(
                    "worker counts must be positive".to_owned(),
                ));
            }
            Ok(count)
        })
        .collect()
}

fn parse_usize(value: &str) -> Result<usize, BenchError> {
    value
        .parse::<usize>()
        .map_err(|error| BenchError::Config(format!("invalid integer {value:?}: {error}")))
}

fn output_len(dimensions: (u32, u32)) -> Result<usize, BenchError> {
    usize::try_from(dimensions.0)
        .ok()
        .and_then(|width| {
            usize::try_from(dimensions.1)
                .ok()
                .map(|height| width * height)
        })
        .and_then(|pixels| pixels.checked_mul(RGBA_CHANNELS))
        .ok_or_else(|| BenchError::Config(format!("output dimensions overflow: {dimensions:?}")))
}

fn image_dimensions(
    dimensions: (u32, u32),
) -> Result<ditherette_wasm::image::ImageDimensions, BenchError> {
    ditherette_wasm::image::ImageDimensions::new(dimensions.0, dimensions.1)
        .map_err(|error| BenchError::Config(format!("invalid dimensions {dimensions:?}: {error}")))
}
