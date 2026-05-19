use std::{
    env,
    fs::{create_dir_all, write},
    hint::black_box,
    path::{Path, PathBuf},
    time::Instant,
};

use ditherette_wasm::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        area::{resize_rgba_area_scalar_into, resize_rgba_area_with_row_band_tiling_into},
        cpu_tiling::{plan_row_bands, RowBandPlan, RowBandTiling, DEFAULT_ROW_BAND_TILING},
        nearest::{resize_rgba_nearest_scalar_into, resize_rgba_nearest_with_row_band_tiling_into},
    },
};
use image::ImageReader;

const DEFAULT_ITERATIONS: usize = 12;
const DEFAULT_WARMUP_ITERATIONS: usize = 2;
const RESIZE_SCALES: [Scale; 33] = [
    Scale::new("2x", 2.0),
    Scale::new("1.9x", 1.9),
    Scale::new("1.8x", 1.8),
    Scale::new("1.7x", 1.7),
    Scale::new("1.6x", 1.6),
    Scale::new("1.5x", 1.5),
    Scale::new("1.4x", 1.4),
    Scale::new("1.3x", 1.3),
    Scale::new("1.2x", 1.2),
    Scale::new("1.1x", 1.1),
    Scale::new("1.05x", 1.05),
    Scale::new("1.04x", 1.04),
    Scale::new("1.03x", 1.03),
    Scale::new("1.02x", 1.02),
    Scale::new("1.01x", 1.01),
    Scale::new("1x", 1.0),
    Scale::new("0.99x", 0.99),
    Scale::new("0.98x", 0.98),
    Scale::new("0.97x", 0.97),
    Scale::new("0.96x", 0.96),
    Scale::new("0.95x", 0.95),
    Scale::new("0.9x", 0.9),
    Scale::new("0.8x", 0.8),
    Scale::new("0.7x", 0.7),
    Scale::new("0.6x", 0.6),
    Scale::new("0.5x", 0.5),
    Scale::new("0.4x", 0.4),
    Scale::new("0.3x", 0.3),
    Scale::new("0.25x", 0.25),
    Scale::new("0.2x", 0.2),
    Scale::new("0.125x", 0.125),
    Scale::new("0.1x", 0.1),
    Scale::new("0.0625x", 0.0625),
];
const MAX_WORKERS: [usize; 4] = [1, 2, 4, 8];
const MIN_PIXELS_PER_BAND: [usize; 4] = [64_000, 128_000, 256_000, 512_000];
const MIN_ROWS_PER_BAND: [usize; 4] = [64, 128, 192, 384];
const GRID_MIN_PARALLEL_OUTPUT_PIXELS: usize = 0;

fn main() -> Result<(), String> {
    let options = Options::parse(env::args().skip(1).collect())?;
    let target = TilingTarget::parse(&options.target)?;
    let fixture = load_fixture(&options.image)?;
    let output = options
        .output
        .unwrap_or_else(|| default_output_path(&target));
    let cases = run_sweep(
        &fixture,
        target,
        options.iterations,
        options.warmup_iterations,
    )?;
    let report = SweepReport {
        target,
        fixture_name: fixture
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_owned(),
        source_dimensions: fixture.dimensions,
        iterations: options.iterations,
        warmup_iterations: options.warmup_iterations,
        cases,
    };

    if let Some(parent) = output.parent() {
        create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    write(&output, report.to_json())
        .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    println!("wrote {}", output.display());
    print_summary(&report);
    Ok(())
}

fn run_sweep(
    fixture: &Fixture,
    target: TilingTarget,
    iterations: usize,
    warmup_iterations: usize,
) -> Result<Vec<SweepCase>, String> {
    let mut cases = Vec::new();
    let tiling_modes = tiling_modes();
    let cases_per_scale = tiling_modes.len() + 1;
    let total_cases = RESIZE_SCALES.len() * cases_per_scale;
    let timing = TimingConfig {
        iterations,
        warmup_iterations,
    };

    eprintln!(
        "starting tiling sweep: target={target} fixture={} source={}x{} scales={} cases_per_scale={} total_cases={} iterations={} warmup={}",
        fixture.path.display(),
        fixture.dimensions.width(),
        fixture.dimensions.height(),
        RESIZE_SCALES.len(),
        cases_per_scale,
        total_cases,
        iterations,
        warmup_iterations,
    );

    for (scale_index, scale) in RESIZE_SCALES.iter().copied().enumerate() {
        let output_dimensions = scale.dimensions_for(fixture.dimensions);
        let output_len = rgba::checked_rgba_byte_len(output_dimensions)
            .map_err(|error| format!("invalid output size: {error}"))?;
        let output = ResizeOutputCase {
            scale,
            dimensions: output_dimensions,
            byte_len: output_len,
        };
        let scale_case_start = scale_index * cases_per_scale;

        eprintln!(
            "scale {}/{}: {} -> {}x{} ({} cases)",
            scale_index + 1,
            RESIZE_SCALES.len(),
            scale.label,
            output_dimensions.width(),
            output_dimensions.height(),
            cases_per_scale,
        );

        let scalar = run_logged_case(
            fixture,
            target,
            output,
            CaseMode::Scalar,
            timing,
            scale_case_start + 1,
            total_cases,
        )?;
        let baseline_median = scalar.stats.median;
        cases.push(scalar);

        for (mode_index, mode) in tiling_modes.iter().copied().enumerate() {
            let mut case = run_logged_case(
                fixture,
                target,
                output,
                mode,
                timing,
                scale_case_start + mode_index + 2,
                total_cases,
            )?;
            case.speedup_vs_scalar = if case.stats.median == 0 {
                None
            } else {
                Some(baseline_median as f64 / case.stats.median as f64)
            };
            cases.push(case);
        }
    }

    eprintln!("finished tiling sweep: {total_cases} cases");
    Ok(cases)
}

fn run_logged_case(
    fixture: &Fixture,
    target: TilingTarget,
    output: ResizeOutputCase,
    mode: CaseMode,
    timing: TimingConfig,
    case_index: usize,
    total_cases: usize,
) -> Result<SweepCase, String> {
    let resolved = mode.tiling().map(|config| {
        plan_row_bands(
            output.dimensions.width() as usize,
            output.dimensions.height() as usize,
            config,
        )
    });
    eprintln!(
        "  case {case_index}/{total_cases}: {} {} -> {}",
        output.scale.label,
        mode.describe(),
        resolved
            .map(format_row_band_plan)
            .unwrap_or_else(|| "scalar".to_owned()),
    );

    let case = run_case(fixture, target, output, mode, timing)?;
    eprintln!(
        "  done {case_index}/{total_cases}: {} {} median={} p95={}",
        output.scale.label,
        mode.name(),
        format_duration_ns(case.stats.median),
        format_duration_ns(case.stats.p95),
    );
    Ok(case)
}

fn run_case(
    fixture: &Fixture,
    target: TilingTarget,
    output: ResizeOutputCase,
    mode: CaseMode,
    timing: TimingConfig,
) -> Result<SweepCase, String> {
    let mut output_rgba = vec![0; output.byte_len];
    let tiling = mode.tiling();

    for _ in 0..timing.warmup_iterations {
        run_target(
            target,
            mode,
            &fixture.rgba,
            fixture.dimensions,
            output.dimensions,
            &mut output_rgba,
        )?;
        black_box(&output_rgba);
    }

    let mut timings = Vec::with_capacity(timing.iterations);
    for _ in 0..timing.iterations {
        let start = Instant::now();
        run_target(
            target,
            mode,
            black_box(&fixture.rgba),
            fixture.dimensions,
            output.dimensions,
            black_box(&mut output_rgba),
        )?;
        black_box(&output_rgba);
        timings.push(start.elapsed().as_nanos());
    }

    let resolved = tiling.map(|config| {
        plan_row_bands(
            output.dimensions.width() as usize,
            output.dimensions.height() as usize,
            config,
        )
    });

    Ok(SweepCase {
        scale_label: output.scale.label,
        output_dimensions: output.dimensions,
        mode_name: mode.name(),
        tiling,
        resolved,
        timings_ns: timings.clone(),
        stats: Stats::from_timings(&timings),
        speedup_vs_scalar: None,
    })
}

fn run_target(
    target: TilingTarget,
    mode: CaseMode,
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), String> {
    match (target.kernel, mode) {
        (TilingKernel::Nearest, CaseMode::Scalar) => resize_rgba_nearest_scalar_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )
        .map_err(format_processing_error),
        (TilingKernel::Nearest, mode) => resize_rgba_nearest_with_row_band_tiling_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
            mode.tiling()
                .expect("nearest tiling mode should provide row-band config"),
            mode.fallback_to_scalar(),
        )
        .map_err(format_processing_error),
        (TilingKernel::Area | TilingKernel::Box, CaseMode::Scalar) => resize_rgba_area_scalar_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )
        .map_err(format_processing_error),
        (TilingKernel::Area | TilingKernel::Box, mode) => {
            resize_rgba_area_with_row_band_tiling_into(
                source_rgba,
                source_dimensions,
                output_dimensions,
                output_rgba,
                mode.tiling()
                    .expect("area tiling mode should provide row-band config"),
            )
            .map_err(format_processing_error)
        }
        (TilingKernel::Planned(name), _) => Err(format!(
            "resize:{name}:tiling sweep is planned but not implemented yet"
        )),
    }
}

fn tiling_modes() -> Vec<CaseMode> {
    let mut modes = vec![CaseMode::DefaultTiling, CaseMode::ForcedOneBand];

    for max_workers in MAX_WORKERS {
        for min_pixels_per_band in MIN_PIXELS_PER_BAND {
            for min_rows_per_band in MIN_ROWS_PER_BAND {
                modes.push(CaseMode::Tiling(RowBandTiling::new(
                    GRID_MIN_PARALLEL_OUTPUT_PIXELS,
                    min_pixels_per_band,
                    min_rows_per_band,
                    max_workers,
                )));
            }
        }
    }

    modes
}

fn load_fixture(path: &Path) -> Result<Fixture, String> {
    let image = ImageReader::open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?
        .decode()
        .map_err(|error| format!("failed to decode {}: {error}", path.display()))?
        .to_rgba8();
    let dimensions = ImageDimensions::new(image.width(), image.height())
        .map_err(|error| format!("invalid fixture dimensions: {error}"))?;

    Ok(Fixture {
        path: path.to_path_buf(),
        dimensions,
        rgba: image.into_raw(),
    })
}

fn print_summary(report: &SweepReport) {
    println!("\n## Tiling sweep report");
    println!(
        "target={} fixture={} source={}x{} scales={} cases={} iterations={} warmup={}",
        report.target,
        report.fixture_name,
        report.source_dimensions.width(),
        report.source_dimensions.height(),
        RESIZE_SCALES.len(),
        report.cases.len(),
        report.iterations,
        report.warmup_iterations,
    );
    println!("\n### Per-scale best");
    println!("| scale | output | scalar | default | default speedup | best | best speedup | best config |");
    println!("|---|---:|---:|---:|---:|---:|---:|---|");

    for scale in RESIZE_SCALES {
        let scale_cases = cases_for_scale(report, scale.label);
        let Some(scalar) = scale_cases
            .iter()
            .copied()
            .find(|case| case.mode_name == "scalar")
        else {
            continue;
        };
        let Some(default_tiling) = scale_cases
            .iter()
            .copied()
            .find(|case| case.mode_name == "default-tiling")
        else {
            continue;
        };
        let Some(best_tiling) = scale_cases
            .iter()
            .copied()
            .filter(|case| case.mode_name != "scalar")
            .min_by_key(|case| case.stats.median)
        else {
            continue;
        };

        println!(
            "| {} | {}x{} | {} | {} | {} | {} | {} | `{}` |",
            scale.label,
            scalar.output_dimensions.width(),
            scalar.output_dimensions.height(),
            format_duration_ns(scalar.stats.median),
            format_duration_ns(default_tiling.stats.median),
            format_speedup(scalar.stats.median, default_tiling.stats.median),
            format_duration_ns(best_tiling.stats.median),
            format_speedup(scalar.stats.median, best_tiling.stats.median),
            format_case_config(best_tiling),
        );
    }

    println!("\n### Default tiling loses to scalar");
    let mut printed_loss = false;
    for scale in RESIZE_SCALES {
        let scale_cases = cases_for_scale(report, scale.label);
        let Some(scalar) = scale_cases
            .iter()
            .copied()
            .find(|case| case.mode_name == "scalar")
        else {
            continue;
        };
        let Some(default_tiling) = scale_cases
            .iter()
            .copied()
            .find(|case| case.mode_name == "default-tiling")
        else {
            continue;
        };
        if default_tiling.stats.median <= scalar.stats.median {
            continue;
        }
        printed_loss = true;
        let best_tiling = scale_cases
            .iter()
            .copied()
            .filter(|case| case.mode_name != "scalar")
            .min_by_key(|case| case.stats.median)
            .expect("scale with default tiling should have at least one tiled case");
        println!(
            "- {}: default {} vs scalar {} ({}); best {} ({}) `{}`",
            scale.label,
            format_duration_ns(default_tiling.stats.median),
            format_duration_ns(scalar.stats.median),
            format_speedup(scalar.stats.median, default_tiling.stats.median),
            format_duration_ns(best_tiling.stats.median),
            format_speedup(scalar.stats.median, best_tiling.stats.median),
            format_case_config(best_tiling),
        );
    }
    if !printed_loss {
        println!("- none");
    }
}

fn cases_for_scale<'a>(report: &'a SweepReport, scale_label: &str) -> Vec<&'a SweepCase> {
    report
        .cases
        .iter()
        .filter(|case| case.scale_label == scale_label)
        .collect()
}

fn format_speedup(scalar_median: u128, candidate_median: u128) -> String {
    if candidate_median == 0 {
        return "—".to_owned();
    }
    format!("{:.2}x", scalar_median as f64 / candidate_median as f64)
}

fn format_case_config(case: &SweepCase) -> String {
    if case.mode_name == "scalar" {
        return "scalar".to_owned();
    }

    let tiling = case
        .tiling
        .map(|tiling| {
            format!(
                "p{} r{} w{}",
                tiling.min_pixels_per_band, tiling.min_rows_per_band, tiling.max_workers,
            )
        })
        .unwrap_or_else(|| "no-tiling".to_owned());
    let resolved = case
        .resolved
        .map(|plan| {
            format!(
                "bands={} workers={} band_h={}",
                plan.band_count, plan.worker_count, plan.band_height,
            )
        })
        .unwrap_or_else(|| "unplanned".to_owned());

    format!("{} {tiling} {resolved}", case.mode_name)
}

fn default_output_path(target: &TilingTarget) -> PathBuf {
    PathBuf::from(format!(
        "benchmark-results/tiling-sweep-{}-{}.json",
        target.stage, target.kernel
    ))
}

fn default_fixture_path() -> PathBuf {
    PathBuf::from("benchmark-fixtures/Celeste_box_art_full.png")
}

fn format_processing_error(error: ProcessingError) -> String {
    format!("processing failed: {error}")
}

fn format_duration_ns(nanoseconds: u128) -> String {
    if nanoseconds >= 1_000_000 {
        format!("{:.3}ms", nanoseconds as f64 / 1_000_000.0)
    } else if nanoseconds >= 1_000 {
        format!("{:.3}µs", nanoseconds as f64 / 1_000.0)
    } else {
        format!("{nanoseconds}ns")
    }
}

fn format_tiling(tiling: RowBandTiling) -> String {
    format!(
        "min_parallel_pixels={} min_pixels_per_band={} min_rows_per_band={} max_workers={}",
        tiling.min_parallel_output_pixels,
        tiling.min_pixels_per_band,
        tiling.min_rows_per_band,
        tiling.max_workers,
    )
}

fn format_row_band_plan(plan: RowBandPlan) -> String {
    format!(
        "bands={} workers={} tile={}x{}",
        plan.band_count, plan.worker_count, plan.output_width, plan.band_height,
    )
}

#[derive(Debug)]
struct Options {
    target: String,
    image: PathBuf,
    output: Option<PathBuf>,
    iterations: usize,
    warmup_iterations: usize,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut target = None;
        let mut image = default_fixture_path();
        let mut output = None;
        let mut iterations = DEFAULT_ITERATIONS;
        let mut warmup_iterations = DEFAULT_WARMUP_ITERATIONS;
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--target" => {
                    target = Some(value_after(&args, index, "--target")?);
                    index += 2;
                }
                "--image" => {
                    image = PathBuf::from(value_after(&args, index, "--image")?);
                    index += 2;
                }
                "--output" => {
                    output = Some(PathBuf::from(value_after(&args, index, "--output")?));
                    index += 2;
                }
                "--iterations" => {
                    iterations =
                        parse_usize(&value_after(&args, index, "--iterations")?, "--iterations")?;
                    index += 2;
                }
                "--warmup-iterations" => {
                    warmup_iterations = parse_usize(
                        &value_after(&args, index, "--warmup-iterations")?,
                        "--warmup-iterations",
                    )?;
                    index += 2;
                }
                "--help" | "-h" => {
                    println!("{}", help_text());
                    std::process::exit(0);
                }
                value => return Err(format!("unknown argument `{value}`\n\n{}", help_text())),
            }
        }

        Ok(Self {
            target: target.ok_or_else(|| format!("missing --target\n\n{}", help_text()))?,
            image,
            output,
            iterations,
            warmup_iterations,
        })
    }
}

fn value_after(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_usize(value: &str, flag: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} must be a positive integer, got `{value}`"))
}

fn help_text() -> &'static str {
    "Usage: pnpm bench:tiling-sweep --target resize:nearest [options]\n\nOptions:\n  --target TARGET             Tiling target, e.g. resize:nearest or resize:area.\n  --image FILE                PNG fixture to decode before sweeping.\n  --output FILE               JSON result path.\n  --iterations N              Timed iterations per case.\n  --warmup-iterations N       Warmup iterations per case."
}

#[derive(Debug, Clone, Copy)]
struct TilingTarget {
    stage: &'static str,
    kernel: TilingKernel,
}

impl TilingTarget {
    fn parse(value: &str) -> Result<Self, String> {
        let Some((stage, kernel)) = value.split_once(':') else {
            return Err(format!("target must use <stage>:<kernel>, got `{value}`"));
        };
        if stage != "resize" {
            return Err(format!(
                "tiling target stage `{stage}` is planned but not implemented yet"
            ));
        }

        let kernel = match kernel {
            "nearest" => TilingKernel::Nearest,
            "area" => TilingKernel::Area,
            "box" => TilingKernel::Box,
            "bilinear" => TilingKernel::Planned("bilinear"),
            "trilinear" => TilingKernel::Planned("trilinear"),
            "bicubic" => TilingKernel::Planned("bicubic"),
            "lanczos2" => TilingKernel::Planned("lanczos2"),
            "lanczos2_scale_aware" => TilingKernel::Planned("lanczos2_scale_aware"),
            "lanczos3" => TilingKernel::Planned("lanczos3"),
            "lanczos3_scale_aware" => TilingKernel::Planned("lanczos3_scale_aware"),
            "antialias" => TilingKernel::Planned("antialias"),
            _ => return Err(format!("unknown resize tiling target `{kernel}`")),
        };

        Ok(Self {
            stage: "resize",
            kernel,
        })
    }
}

impl std::fmt::Display for TilingTarget {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.stage, self.kernel)
    }
}

#[derive(Debug, Clone, Copy)]
enum TilingKernel {
    Nearest,
    Area,
    Box,
    Planned(&'static str),
}

impl std::fmt::Display for TilingKernel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nearest => formatter.write_str("nearest"),
            Self::Area => formatter.write_str("area"),
            Self::Box => formatter.write_str("box"),
            Self::Planned(name) => formatter.write_str(name),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CaseMode {
    Scalar,
    DefaultTiling,
    ForcedOneBand,
    Tiling(RowBandTiling),
}

impl CaseMode {
    fn name(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::DefaultTiling => "default-tiling",
            Self::ForcedOneBand => "forced-one-band",
            Self::Tiling(_) => "tiling",
        }
    }

    fn describe(self) -> String {
        match self {
            Self::Scalar => "scalar".to_owned(),
            Self::DefaultTiling => {
                format!("default-tiling {}", format_tiling(DEFAULT_ROW_BAND_TILING))
            }
            Self::ForcedOneBand => format!(
                "forced-one-band {}",
                format_tiling(RowBandTiling::new(usize::MAX, usize::MAX, 192, 1))
            ),
            Self::Tiling(tiling) => format!("tiling {}", format_tiling(tiling)),
        }
    }

    fn tiling(self) -> Option<RowBandTiling> {
        match self {
            Self::Scalar => None,
            Self::DefaultTiling => Some(DEFAULT_ROW_BAND_TILING),
            Self::ForcedOneBand => Some(RowBandTiling::new(usize::MAX, usize::MAX, 192, 1)),
            Self::Tiling(tiling) => Some(tiling),
        }
    }

    fn fallback_to_scalar(self) -> bool {
        matches!(self, Self::DefaultTiling)
    }
}

#[derive(Debug, Clone, Copy)]
struct ResizeOutputCase {
    scale: Scale,
    dimensions: ImageDimensions,
    byte_len: usize,
}

#[derive(Debug, Clone, Copy)]
struct TimingConfig {
    iterations: usize,
    warmup_iterations: usize,
}

#[derive(Debug)]
struct Fixture {
    path: PathBuf,
    dimensions: ImageDimensions,
    rgba: Vec<u8>,
}

#[derive(Debug)]
struct SweepReport {
    target: TilingTarget,
    fixture_name: String,
    source_dimensions: ImageDimensions,
    iterations: usize,
    warmup_iterations: usize,
    cases: Vec<SweepCase>,
}

impl SweepReport {
    fn to_json(&self) -> String {
        let cases = self
            .cases
            .iter()
            .map(SweepCase::to_json)
            .collect::<Vec<_>>()
            .join(",\n");
        format!(
            "{{\n  \"target\": \"{}\",\n  \"fixture\": \"{}\",\n  \"source\": {{ \"width\": {}, \"height\": {} }},\n  \"iterations\": {},\n  \"warmupIterations\": {},\n  \"cases\": [\n{}\n  ]\n}}\n",
            self.target,
            json_string(&self.fixture_name),
            self.source_dimensions.width(),
            self.source_dimensions.height(),
            self.iterations,
            self.warmup_iterations,
            indent(&cases, 4),
        )
    }
}

#[derive(Debug)]
struct SweepCase {
    scale_label: &'static str,
    output_dimensions: ImageDimensions,
    mode_name: &'static str,
    tiling: Option<RowBandTiling>,
    resolved: Option<RowBandPlan>,
    timings_ns: Vec<u128>,
    stats: Stats,
    speedup_vs_scalar: Option<f64>,
}

impl SweepCase {
    fn to_json(&self) -> String {
        format!(
            "{{\n  \"scale\": \"{}\",\n  \"output\": {{ \"width\": {}, \"height\": {} }},\n  \"mode\": \"{}\",\n  \"tiling\": {},\n  \"resolved\": {},\n  \"timingsNs\": [{}],\n  \"statsNs\": {},\n  \"speedupVsScalar\": {}\n}}",
            self.scale_label,
            self.output_dimensions.width(),
            self.output_dimensions.height(),
            self.mode_name,
            tiling_json(self.tiling),
            resolved_json(self.resolved),
            self.timings_ns
                .iter()
                .map(u128::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            self.stats.to_json(),
            optional_f64_json(self.speedup_vs_scalar),
        )
    }
}

#[derive(Debug)]
struct Stats {
    mean: u128,
    median: u128,
    p75: u128,
    p95: u128,
    min: u128,
    max: u128,
}

impl Stats {
    fn from_timings(timings: &[u128]) -> Self {
        let mut sorted = timings.to_vec();
        sorted.sort_unstable();
        let total: u128 = sorted.iter().sum();
        Self {
            mean: total / sorted.len() as u128,
            median: percentile(&sorted, 50),
            p75: percentile(&sorted, 75),
            p95: percentile(&sorted, 95),
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        }
    }

    fn to_json(&self) -> String {
        format!(
            "{{ \"mean\": {}, \"median\": {}, \"p75\": {}, \"p95\": {}, \"min\": {}, \"max\": {} }}",
            self.mean, self.median, self.p75, self.p95, self.min, self.max,
        )
    }
}

fn percentile(sorted: &[u128], percentile_value: usize) -> u128 {
    let index = ((sorted.len() - 1) * percentile_value).div_ceil(100);
    sorted[index]
}

fn optional_f64_json(value: Option<f64>) -> String {
    match value {
        Some(value) if value.is_finite() => format!("{value:.6}"),
        _ => "null".to_owned(),
    }
}

fn tiling_json(tiling: Option<RowBandTiling>) -> String {
    tiling
        .map(|config| {
            format!(
                "{{ \"maxWorkers\": {}, \"minParallelOutputPixels\": {}, \"minPixelsPerBand\": {}, \"minRowsPerBand\": {} }}",
                config.max_workers,
                config.min_parallel_output_pixels,
                config.min_pixels_per_band,
                config.min_rows_per_band,
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

fn resolved_json(resolved: Option<RowBandPlan>) -> String {
    resolved
        .map(|plan| {
            format!(
                "{{ \"availableLogicalThreads\": {}, \"workerCount\": {}, \"bandCount\": {}, \"tile\": {{ \"width\": {}, \"height\": {} }} }}",
                plan.available_logical_threads,
                plan.worker_count,
                plan.band_count,
                plan.output_width,
                plan.band_height,
            )
        })
        .unwrap_or_else(|| "null".to_owned())
}

fn indent(value: &str, spaces: usize) -> String {
    let padding = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{padding}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
