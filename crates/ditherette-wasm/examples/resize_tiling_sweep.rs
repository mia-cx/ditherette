use std::{
    fs::{self, File},
    hint::black_box,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use ditherette_wasm::{
    image::{rgba, ImageDimensions},
    resize::{
        cpu_tiling::{plan_row_bands, RowBandPlan, RowBandTiling},
        nearest::{
            resize_rgba_nearest_scalar_into, resize_rgba_nearest_with_forced_tiling_into,
            resize_rgba_nearest_with_tiling_into,
        },
    },
};
use image::ImageReader;

const SCALES: [Scale; 14] = [
    Scale::new("2x", 2.0),
    Scale::new("1.5x", 1.5),
    Scale::new("1.25x", 1.25),
    Scale::new("1x", 1.0),
    Scale::new("0.95x", 0.95),
    Scale::new("0.875x", 0.875),
    Scale::new("0.75x", 0.75),
    Scale::new("0.625x", 0.625),
    Scale::new("0.5x", 0.5),
    Scale::new("0.375x", 0.375),
    Scale::new("0.3125x", 0.3125),
    Scale::new("0.25x", 0.25),
    Scale::new("0.1875x", 0.1875),
    Scale::new("0.125x", 0.125),
];
const MAX_WORKERS: [usize; 6] = [1, 2, 4, 8, 12, 16];
const MIN_PIXELS_PER_BAND: [usize; 5] = [64_000, 128_000, 256_000, 512_000, 1_000_000];
const MIN_ROWS_PER_BAND: usize = 192;
const DEFAULT_ITERATIONS: usize = 12;
const DEFAULT_WARMUP_ITERATIONS: usize = 2;
const OUTPUT_JSON: &str = "benchmark-results/resize-tiling-sweep-nearest.json";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iterations = env_usize("RESIZE_TILING_SWEEP_ITERATIONS", DEFAULT_ITERATIONS);
    let warmup_iterations = env_usize(
        "RESIZE_TILING_SWEEP_WARMUP_ITERATIONS",
        DEFAULT_WARMUP_ITERATIONS,
    );
    let fixture = load_celeste_fixture();
    let mut cases = Vec::new();

    for scale in SCALES {
        let output_dimensions = scale.dimensions_for(fixture.dimensions);
        let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions)?;
        let scalar_case = measure_case(
            &fixture,
            CaseConfig {
                scale,
                output_dimensions,
                output_byte_len,
                mode: SweepMode::Scalar,
                tiling: None,
                iterations,
                warmup_iterations,
            },
        )?;
        let scalar_median = scalar_case.stats.median;
        cases.push(scalar_case);

        let one_band_config = RowBandTiling::new(usize::MAX, usize::MAX, MIN_ROWS_PER_BAND, 1);
        cases.push(measure_case(
            &fixture,
            CaseConfig {
                scale,
                output_dimensions,
                output_byte_len,
                mode: SweepMode::ForcedOneBand,
                tiling: Some(one_band_config),
                iterations,
                warmup_iterations,
            },
        )?);

        for max_workers in MAX_WORKERS {
            for min_pixels_per_band in MIN_PIXELS_PER_BAND {
                let config =
                    RowBandTiling::new(0, min_pixels_per_band, MIN_ROWS_PER_BAND, max_workers);
                cases.push(measure_case(
                    &fixture,
                    CaseConfig {
                        scale,
                        output_dimensions,
                        output_byte_len,
                        mode: SweepMode::Tiling { scalar_median },
                        tiling: Some(config),
                        iterations,
                        warmup_iterations,
                    },
                )?);
            }
        }
    }

    print_table(&cases);
    write_json(&fixture, iterations, warmup_iterations, &cases)?;

    Ok(())
}

fn measure_case(
    fixture: &RgbaFixture,
    config: CaseConfig,
) -> Result<SweepCase, Box<dyn std::error::Error>> {
    let mut output_rgba = vec![0; config.output_byte_len];
    let plan = config.tiling.map(|tiling| {
        plan_row_bands(
            config.output_dimensions.width() as usize,
            config.output_dimensions.height() as usize,
            tiling,
        )
    });

    for _ in 0..config.warmup_iterations {
        run_resize(
            fixture,
            config.output_dimensions,
            &mut output_rgba,
            config.mode,
            config.tiling,
        )?;
        black_box(&output_rgba);
    }

    let mut timings = Vec::with_capacity(config.iterations);
    for _ in 0..config.iterations {
        let started = Instant::now();
        run_resize(
            fixture,
            config.output_dimensions,
            &mut output_rgba,
            config.mode,
            config.tiling,
        )?;
        black_box(&output_rgba);
        timings.push(started.elapsed().as_nanos() as u64);
    }

    let stats = Stats::from_timings(&timings);
    let speedup_vs_scalar = config
        .mode
        .scalar_median()
        .map(|scalar_median| scalar_median as f64 / stats.median as f64);

    Ok(SweepCase {
        scale: config.scale,
        output_dimensions: config.output_dimensions,
        mode: config.mode.name(),
        config: config.tiling,
        plan,
        timings,
        stats,
        speedup_vs_scalar,
    })
}

fn run_resize(
    fixture: &RgbaFixture,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    mode: SweepMode,
    config: Option<RowBandTiling>,
) -> Result<(), Box<dyn std::error::Error>> {
    match mode {
        SweepMode::Scalar => resize_rgba_nearest_scalar_into(
            black_box(&fixture.rgba),
            fixture.dimensions,
            output_dimensions,
            black_box(output_rgba),
        )?,
        SweepMode::ForcedOneBand => resize_rgba_nearest_with_forced_tiling_into(
            black_box(&fixture.rgba),
            fixture.dimensions,
            output_dimensions,
            black_box(output_rgba),
            config.expect("forced one-band case should provide tiling config"),
        )?,
        SweepMode::Tiling { .. } => resize_rgba_nearest_with_tiling_into(
            black_box(&fixture.rgba),
            fixture.dimensions,
            output_dimensions,
            black_box(output_rgba),
            config.expect("tiling case should provide tiling config"),
        )?,
    }

    Ok(())
}

fn print_table(cases: &[SweepCase]) {
    println!(
        "scale\tsize\tmode\tmax_workers\tmin_band_px\tworkers\tbands\ttile\tmedian\tp75\tspeedup"
    );

    for case in cases {
        let config = case.config;
        let plan = case.plan;
        println!(
            "{}\t{}x{}\t{}\t{}\t{}\t{}\t{}\t{}x{}\t{}\t{}\t{}",
            case.scale.label,
            case.output_dimensions.width(),
            case.output_dimensions.height(),
            case.mode,
            config.map_or("—".to_string(), |config| config.max_workers.to_string()),
            config.map_or("—".to_string(), |config| config
                .min_pixels_per_band
                .to_string()),
            plan.map_or("—".to_string(), |plan| plan.worker_count.to_string()),
            plan.map_or("—".to_string(), |plan| plan.band_count.to_string()),
            plan.map_or(case.output_dimensions.width().to_string(), |plan| plan
                .output_width
                .to_string()),
            plan.map_or(case.output_dimensions.height().to_string(), |plan| plan
                .band_height
                .to_string()),
            format_duration(case.stats.median),
            format_duration(case.stats.p75),
            case.speedup_vs_scalar
                .map_or("—".to_string(), |speedup| format!("{speedup:.3}×")),
        );
    }
}

fn write_json(
    fixture: &RgbaFixture,
    iterations: usize,
    warmup_iterations: usize,
    cases: &[SweepCase],
) -> io::Result<()> {
    let output_path = repo_root().join(OUTPUT_JSON);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(&output_path)?;
    writeln!(file, "{{")?;
    writeln!(file, "  \"fixture\": \"Celeste_box_art_full.png\",")?;
    writeln!(
        file,
        "  \"source\": {{ \"width\": {}, \"height\": {} }},",
        fixture.dimensions.width(),
        fixture.dimensions.height()
    )?;
    writeln!(file, "  \"filter\": \"nearest\",")?;
    writeln!(file, "  \"iterations\": {iterations},")?;
    writeln!(file, "  \"warmupIterations\": {warmup_iterations},")?;
    writeln!(file, "  \"cases\": [")?;

    for (index, case) in cases.iter().enumerate() {
        write_case_json(&mut file, case, index + 1 == cases.len())?;
    }

    writeln!(file, "  ]")?;
    writeln!(file, "}}")?;
    eprintln!("wrote {}", output_path.display());

    Ok(())
}

fn write_case_json(file: &mut File, case: &SweepCase, is_last: bool) -> io::Result<()> {
    writeln!(file, "    {{")?;
    writeln!(file, "      \"scale\": \"{}\",", case.scale.label)?;
    writeln!(
        file,
        "      \"output\": {{ \"width\": {}, \"height\": {} }},",
        case.output_dimensions.width(),
        case.output_dimensions.height()
    )?;
    writeln!(file, "      \"mode\": \"{}\",", case.mode)?;

    if let Some(config) = case.config {
        writeln!(file, "      \"maxWorkers\": {},", config.max_workers)?;
        writeln!(
            file,
            "      \"minParallelOutputPixels\": {},",
            config.min_parallel_output_pixels
        )?;
        writeln!(
            file,
            "      \"minPixelsPerBand\": {},",
            config.min_pixels_per_band
        )?;
        writeln!(
            file,
            "      \"minRowsPerBand\": {},",
            config.min_rows_per_band
        )?;
    } else {
        writeln!(file, "      \"maxWorkers\": null,")?;
        writeln!(file, "      \"minParallelOutputPixels\": null,")?;
        writeln!(file, "      \"minPixelsPerBand\": null,")?;
        writeln!(file, "      \"minRowsPerBand\": null,")?;
    }

    if let Some(plan) = case.plan {
        writeln!(
            file,
            "      \"resolved\": {{ \"availableLogicalThreads\": {}, \"workerCount\": {}, \"bandCount\": {}, \"tile\": {{ \"width\": {}, \"height\": {} }} }},",
            plan.available_logical_threads,
            plan.worker_count,
            plan.band_count,
            plan.output_width,
            plan.band_height
        )?;
    } else {
        writeln!(file, "      \"resolved\": null,")?;
    }

    writeln!(file, "      \"timingsNs\": {},", json_array(&case.timings))?;
    writeln!(
        file,
        "      \"statsNs\": {{ \"mean\": {}, \"median\": {}, \"p75\": {}, \"p95\": {}, \"min\": {}, \"max\": {} }},",
        case.stats.mean,
        case.stats.median,
        case.stats.p75,
        case.stats.p95,
        case.stats.min,
        case.stats.max,
    )?;
    match case.speedup_vs_scalar {
        Some(speedup) => writeln!(file, "      \"speedupVsScalar\": {speedup:.6}")?,
        None => writeln!(file, "      \"speedupVsScalar\": null")?,
    }
    write!(file, "    }}")?;
    if !is_last {
        writeln!(file, ",")?;
    } else {
        writeln!(file)?;
    }

    Ok(())
}

fn json_array(values: &[u64]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[derive(Debug, Clone, Copy)]
struct CaseConfig {
    scale: Scale,
    output_dimensions: ImageDimensions,
    output_byte_len: usize,
    mode: SweepMode,
    tiling: Option<RowBandTiling>,
    iterations: usize,
    warmup_iterations: usize,
}

#[derive(Debug, Clone, Copy)]
enum SweepMode {
    Scalar,
    ForcedOneBand,
    Tiling { scalar_median: u64 },
}

impl SweepMode {
    fn name(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::ForcedOneBand => "forced-one-band",
            Self::Tiling { .. } => "tiling",
        }
    }

    fn scalar_median(self) -> Option<u64> {
        match self {
            Self::Scalar | Self::ForcedOneBand => None,
            Self::Tiling { scalar_median } => Some(scalar_median),
        }
    }
}

#[derive(Debug)]
struct SweepCase {
    scale: Scale,
    output_dimensions: ImageDimensions,
    mode: &'static str,
    config: Option<RowBandTiling>,
    plan: Option<RowBandPlan>,
    timings: Vec<u64>,
    stats: Stats,
    speedup_vs_scalar: Option<f64>,
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

#[derive(Debug)]
struct RgbaFixture {
    dimensions: ImageDimensions,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct Stats {
    mean: u64,
    median: u64,
    p75: u64,
    p95: u64,
    min: u64,
    max: u64,
}

impl Stats {
    fn from_timings(timings: &[u64]) -> Self {
        let mut sorted = timings.to_vec();
        sorted.sort_unstable();
        let sum: u128 = sorted.iter().map(|value| u128::from(*value)).sum();

        Self {
            mean: (sum / sorted.len() as u128) as u64,
            median: percentile(&sorted, 50),
            p75: percentile(&sorted, 75),
            p95: percentile(&sorted, 95),
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        }
    }
}

fn percentile(sorted: &[u64], percentile: u32) -> u64 {
    if sorted.len() == 1 {
        return sorted[0];
    }

    let rank = percentile as f64 / 100.0 * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let weight = rank - lower as f64;

    (sorted[lower] as f64 * (1.0 - weight) + sorted[upper] as f64 * weight).round() as u64
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
    repo_root().join("benchmark-fixtures/Celeste_box_art_full.png")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should live under crates/ditherette-wasm")
        .to_path_buf()
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn scaled_dimension(source_dimension: u32, scale: f64) -> u32 {
    ((f64::from(source_dimension) * scale).floor() as u32).max(1)
}

fn format_duration(nanoseconds: u64) -> String {
    if nanoseconds >= 1_000_000 {
        format!("{:.4}ms", nanoseconds as f64 / 1_000_000.0)
    } else if nanoseconds >= 1_000 {
        format!("{:.4}µs", nanoseconds as f64 / 1_000.0)
    } else {
        format!("{nanoseconds}ns")
    }
}
