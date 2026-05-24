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
    time::Instant,
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
    result::SampleStats,
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
const DEFAULT_SAMPLES: usize = 7;
const DEFAULT_WARMUP: usize = 2;
const MAX_DIMENSION_CASES_PER_FIXTURE: usize = 26;
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
    let total_cases = fixtures.len()
        * subjects.len()
        * dimensions.iter().map(Vec::len).sum::<usize>()
        * options.band_heights.len();
    eprintln!(
        "tiling sweep: fixtures={} filters={} dimension_sets={} band_heights={} cases={} samples={} warmup={}",
        fixtures.len(),
        subjects.len(),
        dimensions.iter().map(Vec::len).sum::<usize>(),
        options.band_heights.len(),
        total_cases,
        options.samples,
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
                    options.warmup_iterations,
                )?;

                for &band_height in &options.band_heights {
                    case_index += 1;
                    let tiled = measure_subject_row_bands(
                        subject,
                        &fixture.rgba,
                        (fixture.width, fixture.height),
                        *output,
                        band_height,
                        options.samples,
                        options.warmup_iterations,
                    )?;
                    let band_count = output.1.div_ceil(band_height);
                    let pixels_per_band =
                        u64::from(output.0) * u64::from(band_height.min(output.1));
                    let speedup = scalar.stats.median_ns / tiled.stats.median_ns;
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
                        pixels_per_band,
                        scalar_median_ns: scalar.stats.median_ns,
                        tiled_median_ns: tiled.stats.median_ns,
                        speedup,
                        tiled_p95_ns: tiled.stats.p95_ns,
                        tiled_stdev_ns: tiled.stats.stdev_ns,
                        checksum: checksum(&tiled.output),
                    });

                    eprintln!(
                        "{case_index}/{total_cases} {} {} {}x{} band={} speedup={:.3} tiled={}",
                        subject.descriptor.id,
                        fixture.id,
                        output.0,
                        output.1,
                        band_height,
                        speedup,
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
    samples: usize,
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
            samples: flags
                .optional("--sample-size")
                .or_else(|| flags.optional("--samples"))
                .map(parse_usize)
                .transpose()?
                .unwrap_or(DEFAULT_SAMPLES),
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
        .map(|filter| registry.resize_subject(subject_for_filter(filter)))
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
        8,
        16,
        24,
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
    warmup_iterations: usize,
) -> Result<MeasuredOutput, BenchError> {
    let mut output = vec![0; output_len(output_dimensions)?];
    for _ in 0..warmup_iterations {
        run_subject(
            subject,
            source,
            &mut output,
            source_dimensions,
            output_dimensions,
        )?;
        black_box(&output);
    }
    let mut sample_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        run_subject(
            subject,
            black_box(source),
            black_box(&mut output),
            source_dimensions,
            output_dimensions,
        )?;
        let elapsed = start.elapsed();
        black_box(&output);
        sample_ns.push(elapsed.as_nanos() as f64);
    }
    Ok(MeasuredOutput {
        output,
        stats: SampleStats::from_samples(&sample_ns),
    })
}

fn measure_subject_row_bands(
    subject: &ResizeBenchSubject,
    source: &[u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    band_height: u32,
    samples: usize,
    warmup_iterations: usize,
) -> Result<MeasuredOutput, BenchError> {
    let mut output = vec![0; output_len(output_dimensions)?];
    for _ in 0..warmup_iterations {
        run_subject_row_bands(
            subject,
            source,
            &mut output,
            source_dimensions,
            output_dimensions,
            band_height,
        )?;
        black_box(&output);
    }
    let mut sample_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        run_subject_row_bands(
            subject,
            black_box(source),
            black_box(&mut output),
            source_dimensions,
            output_dimensions,
            band_height,
        )?;
        let elapsed = start.elapsed();
        black_box(&output);
        sample_ns.push(elapsed.as_nanos() as f64);
    }
    Ok(MeasuredOutput {
        output,
        stats: SampleStats::from_samples(&sample_ns),
    })
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

fn run_subject_row_bands(
    subject: &ResizeBenchSubject,
    source: &[u8],
    output: &mut [u8],
    source_dimensions: (u32, u32),
    output_dimensions: (u32, u32),
    band_height: u32,
) -> Result<(), BenchError> {
    let actual_band_height = if band_height == u32::MAX {
        output_dimensions.1
    } else {
        band_height
    };
    let Some(plan) = ditherette_wasm::prod::tiling::RowBandPlan::for_output_height(
        image_dimensions(output_dimensions)?,
        actual_band_height,
    ) else {
        return Err(BenchError::Config(format!(
            "invalid band height {band_height}"
        )));
    };

    for band in plan.bands() {
        let y_start = band.y_start();
        let y_end = band.y_end();
        let band_output_dimensions = (output_dimensions.0, y_end - y_start);
        let mut band_output = vec![0; output_len(band_output_dimensions)?];
        run_subject(
            subject,
            source,
            &mut band_output,
            source_dimensions,
            band_output_dimensions,
        )?;
        copy_band_output(&band_output, output, output_dimensions.0, y_start)?;
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
    writeln!(writer, "fixture,subject,filter,source_width,source_height,output_width,output_height,output_pixels,scale_x,scale_y,band_height,band_count,pixels_per_band,scalar_median_ns,tiled_median_ns,speedup,tiled_p95_ns,tiled_stdev_ns,checksum").map_err(BenchError::io)?;
    for row in rows {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{:.6},{:.6},{},{},{},{:.3},{:.3},{:.6},{:.3},{:.3},{}",
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
    writeln!(writer, "fixture,subject,filter,source_width,source_height,output_width,output_height,output_pixels,best_band_height,best_band_count,best_pixels_per_band,scalar_median_ns,best_tiled_median_ns,best_speedup").map_err(BenchError::io)?;
    for row in rows {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.6}",
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
                    && row.band_height < existing.band_height)
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
    formula: &'static str,
    intercept: f64,
    log_output_pixels: f64,
    log_kernel_cost: f64,
    log_minify: f64,
    rmse_log_pixels_per_band: f64,
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
            .filter(|row| row.subject == subject)
            .collect::<Vec<_>>();
        if subject_rows.len() < 4 {
            continue;
        }
        let coefficients = fit_log_linear(&subject_rows);
        let rmse = curve_rmse(&subject_rows, coefficients);
        curves.push(Curve {
            filter: subject_rows[0].filter.clone(),
            subject,
            samples: subject_rows.len(),
            formula: "target_pixels_per_band = exp(intercept + a*ln(output_pixels) + b*ln(kernel_cost) + c*ln(max(source_w/output_w, source_h/output_h, 1)))",
            intercept: coefficients[0],
            log_output_pixels: coefficients[1],
            log_kernel_cost: coefficients[2],
            log_minify: coefficients[3],
            rmse_log_pixels_per_band: rmse,
        });
    }

    CurveFile {
        schema: "ditherette-tiling-sweep-curves-v1",
        description: "Smooth empirical row-band tiling curves fitted from real fixture dimensions. Rows measure band-sized resize kernel cost through the public subject API; production row-range adapters should validate fitted curves before policy rollout.",
        curves,
    }
}

fn fit_log_linear(rows: &[&SweepRow]) -> [f64; 4] {
    let lambda = 0.001;
    let mut xtx = [[0.0; 4]; 4];
    let mut xty = [0.0; 4];
    for row in rows {
        let x = curve_features(row);
        let y = (row.pixels_per_band.max(1) as f64).ln();
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

fn curve_rmse(rows: &[&SweepRow], coefficients: [f64; 4]) -> f64 {
    let squared = rows
        .iter()
        .map(|row| {
            let predicted = dot(coefficients, curve_features(row));
            let actual = (row.pixels_per_band.max(1) as f64).ln();
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
    writeln!(writer, "\nNote: this command measures row-band cost by running public resize subjects on band-sized outputs and summing those costs. It uses `prod::tiling::RowBandPlan` for the candidate geometry, but final production curves still need validation with true absolute-coordinate row-range adapters.\n").map_err(BenchError::io)?;
    writeln!(writer, "best rows: {}", best_rows.len()).map_err(BenchError::io)?;
    writeln!(writer, "curves: {}\n", curves.curves.len()).map_err(BenchError::io)?;
    writeln!(
        writer,
        "| filter | subject | samples | formula | rmse | coefficients |"
    )
    .map_err(BenchError::io)?;
    writeln!(writer, "| --- | --- | ---: | --- | ---: | --- |").map_err(BenchError::io)?;
    for curve in &curves.curves {
        writeln!(
            writer,
            "| {} | `{}` | {} | target pixels/band | {:.4} | intercept={:.4}, output={:.4}, kernel={:.4}, minify={:.4} |",
            curve.filter,
            curve.subject,
            curve.samples,
            curve.rmse_log_pixels_per_band,
            curve.intercept,
            curve.log_output_pixels,
            curve.log_kernel_cost,
            curve.log_minify,
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
