//! Browser/Wasm resize benchmark transport.
//!
//! Rust owns the benchmark UI and artifact. The Node/Playwright script is only
//! a browser harness that serves fixtures, calls Wasm, and streams JSONL events.

use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use ditherette_bench::lease::Lease;
use serde::Deserialize;

use crate::{
    baseline::{load_scoped_baseline, save_indexed_run, save_scoped_baseline},
    case::ResizeScale,
    compare::attach_accepted_comparisons,
    error::BenchError,
    fixture::Fixture,
    measure::{MeasurementConfig, MeasurementObserver, MeasurementProgress},
    report::{log_perf_start, print_perf_table, MeasurementLogger},
    result::{BenchResult, BenchRun, ComparisonReport, SampleStats},
};

const WASM_MANIFEST: &str = "scripts/ditherette-wasm-bench.toml";
const WASM_HARNESS: &str = "scripts/benchmark-wasm-resize.mjs";

pub(crate) fn wasm_resize_command(lease: &Lease, args: &[String]) -> Result<(), BenchError> {
    let args = strip_leading_separator(args);
    let bench_flags = WasmBenchFlags::parse(args)?;
    let mut command = Command::new("node");
    command
        .arg(WASM_HARNESS)
        .args(with_default_config(&bench_flags.harness_args))
        .arg("--jsonl-events")
        .env("DITHERETTE_BENCH_TRANSPORT", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    run_harness(lease, command, &bench_flags)
}

fn run_harness(
    lease: &Lease,
    command: Command,
    bench_flags: &WasmBenchFlags,
) -> Result<(), BenchError> {
    let mut child = lease.spawn(command).map_err(|error| {
        BenchError::Runtime(format!("failed to start browser/Wasm harness: {error}"))
    })?;

    let stdout = child.take_stdout().ok_or_else(|| {
        BenchError::Runtime("failed to capture browser/Wasm harness stdout".to_owned())
    })?;
    let reader = BufReader::new(stdout);

    let mut state = WasmRunState {
        accepted_baseline: bench_flags.accepted_baseline.clone(),
        ..WasmRunState::default()
    };
    for line in reader.lines() {
        let line = line.map_err(BenchError::io)?;
        if line.trim().is_empty() {
            continue;
        }
        let event = serde_json::from_str::<WasmEvent>(&line).map_err(|error| {
            BenchError::Runtime(format!(
                "browser/Wasm harness emitted invalid JSONL event: {error}: {line}"
            ))
        })?;
        state.handle(event)?;
    }

    let status = child.wait().map_err(BenchError::io)?;
    if !status.success() {
        return Err(BenchError::Runtime(format!(
            "browser/Wasm harness exited with {status}"
        )));
    }

    state.finish(&bench_flags)
}

fn strip_leading_separator(args: &[String]) -> &[String] {
    if args.first().is_some_and(|arg| arg == "--") {
        &args[1..]
    } else {
        args
    }
}

struct WasmBenchFlags {
    harness_args: Vec<String>,
    accepted_baseline: Option<String>,
    save_baseline: Option<String>,
    replacing_baseline: bool,
}

impl WasmBenchFlags {
    fn parse(args: &[String]) -> Result<Self, BenchError> {
        let mut harness_args = Vec::new();
        let mut accepted_baseline = None;
        let mut save_baseline = None;
        let mut replacing_baseline = false;
        let mut no_run = false;
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if let Some(value) = arg.strip_prefix("--baseline=") {
                accepted_baseline = Some(value.to_owned());
                index += 1;
            } else if arg == "--baseline" {
                accepted_baseline = Some(required_value(args, &mut index, "--baseline")?);
            } else if let Some(value) = arg.strip_prefix("--save-baseline=") {
                save_baseline = Some(value.to_owned());
                index += 1;
            } else if arg == "--save-baseline" {
                save_baseline =
                    Some(optional_value(args, &mut index).unwrap_or_else(|| "true".to_owned()));
            } else if let Some(value) = arg.strip_prefix("--replace-baseline=") {
                save_baseline = Some(value.to_owned());
                replacing_baseline = true;
                index += 1;
            } else if arg == "--replace-baseline" {
                save_baseline = Some(required_value(args, &mut index, "--replace-baseline")?);
                replacing_baseline = true;
            } else if arg == "--no-run" {
                no_run = true;
                index += 1;
            } else if let Some((flag, value)) = normalize_harness_alias(args, &mut index)? {
                harness_args.push(flag);
                harness_args.push(value);
            } else if unsupported_bench_flag(arg).is_some() {
                return Err(BenchError::Config(format!(
                    "{arg} is a ditherette-bench flag that is not supported by wasm-resize yet"
                )));
            } else {
                harness_args.push(arg.clone());
                index += 1;
            }
        }
        if no_run {
            return Err(BenchError::Config(
                "wasm-resize does not support --no-run yet; use --replace-baseline NAME to refresh the baseline from a new browser run".to_owned(),
            ));
        }
        Ok(Self {
            harness_args,
            accepted_baseline,
            save_baseline,
            replacing_baseline,
        })
    }
}

fn required_value(args: &[String], index: &mut usize, flag: &str) -> Result<String, BenchError> {
    let value = args
        .get(*index + 1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| BenchError::Config(format!("{flag} requires a value")))?;
    *index += 2;
    Ok(value.clone())
}

fn optional_value(args: &[String], index: &mut usize) -> Option<String> {
    let value = args
        .get(*index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned();
    *index += if value.is_some() { 2 } else { 1 };
    value
}

fn normalize_harness_alias(
    args: &[String],
    index: &mut usize,
) -> Result<Option<(String, String)>, BenchError> {
    let arg = &args[*index];
    let (name, inline_value) = arg
        .split_once('=')
        .map_or((arg.as_str(), None), |(name, value)| (name, Some(value)));
    let Some(flag) = normalized_harness_flag(name) else {
        return Ok(None);
    };
    let value = if let Some(value) = inline_value {
        *index += 1;
        value.to_owned()
    } else {
        required_value(args, index, arg)?
    };
    Ok(Some((flag.to_owned(), value)))
}

fn normalized_harness_flag(flag: &str) -> Option<&'static str> {
    match flag {
        "--samples" | "--measurement-iterations" => Some("--sample-size"),
        "--warmup-iterations" => Some("--warm-up-iterations"),
        "--warmup-time" => Some("--warm-up-time"),
        "--target-sample-ms" => Some("--target-sample-time"),
        _ => None,
    }
}

fn unsupported_bench_flag(flag: &str) -> Option<&str> {
    let name = flag.split_once('=').map_or(flag, |(name, _)| name);
    match name {
        "--oracle"
        | "--correctness"
        | "--max-color-distance"
        | "--max-mean-color-distance"
        | "--max-rms-color-distance"
        | "--allow-correctness-failures"
        | "--output-img"
        | "--save-oracle"
        | "--replace-oracle"
        | "--spec-baseline"
        | "--save-spec-baseline"
        | "--sample-mode"
        | "--cache-state"
        | "--cache-scrub-size"
        | "--inter-sample-delay"
        | "--inter-sample-delay-ms"
        | "--preheat-time"
        | "--preheat-time-ms"
        | "--process-priority" => Some(name),
        _ => None,
    }
}

fn with_default_config(args: &[String]) -> Vec<String> {
    if args
        .iter()
        .any(|arg| arg == "--config" || arg.starts_with("--config="))
    {
        return args.to_vec();
    }
    if args.first().is_some_and(|arg| arg == "run") && args.len() >= 2 {
        let mut with_config = vec![
            args[0].clone(),
            args[1].clone(),
            "--config".to_owned(),
            WASM_MANIFEST.to_owned(),
        ];
        with_config.extend_from_slice(&args[2..]);
        return with_config;
    }
    let mut with_config = vec!["--config".to_owned(), WASM_MANIFEST.to_owned()];
    with_config.extend_from_slice(args);
    with_config
}

#[derive(Default)]
struct WasmRunState {
    measurement: Option<MeasurementConfig>,
    results: Vec<BenchResult>,
    logger: Option<MeasurementLogger>,
    profile: Option<String>,
    accepted_baseline: Option<String>,
    same_run_scalar_comparison: bool,
    domain: Option<String>,
}

impl WasmRunState {
    fn handle(&mut self, event: WasmEvent) -> Result<(), BenchError> {
        match event {
            WasmEvent::Start(event) => self.start(event),
            WasmEvent::WarmupBatch(event) => {
                self.logger(&event.subject, &event.case_id)?
                    .warmup_batch(event.batch_size, event.batch_duration());
                Ok(())
            }
            WasmEvent::WarmupFinished(event) => {
                self.logger(&event.subject, &event.case_id)?
                    .warmup_finished(event.batch_size, event.batch_duration());
                Ok(())
            }
            WasmEvent::MeasurementProgress(event) => {
                let progress = MeasurementProgress {
                    samples_done: event.samples_done,
                    sample_size: event.sample_size,
                    elapsed: duration_ms(event.elapsed_ms),
                    measurement_time: duration_ms(event.measurement_time_ms),
                    total_iterations: event.total_iterations,
                };
                self.logger(&event.subject, &event.case_id)?
                    .measurement_progress(
                        progress,
                        &event.samples_ns,
                        (event.output.width, event.output.height),
                    );
                Ok(())
            }
            WasmEvent::Result(event) => {
                let mut result = event.result.into_bench_result();
                if self.same_run_scalar_comparison {
                    self.attach_same_run_scalar_comparison(&mut result);
                } else if let Some(name) = self.accepted_baseline_name() {
                    self.attach_accepted_comparison(name, &mut result)?;
                }
                if let Some(mut logger) = self.logger.take() {
                    logger.finish(&result);
                } else {
                    let measurement = self.measurement()?;
                    let mut logger =
                        MeasurementLogger::new(&result.subject, &result.case_id, measurement);
                    logger.finish(&result);
                }
                self.results.push(result);
                Ok(())
            }
            WasmEvent::Complete(_) => Ok(()),
        }
    }

    fn start(&mut self, event: StartEvent) -> Result<(), BenchError> {
        self.domain = Some(event.domain.clone());
        self.profile = event.profile;
        self.same_run_scalar_comparison =
            same_run_scalar_comparison_profile(self.profile.as_deref());
        if self.same_run_scalar_comparison {
            self.accepted_baseline = None;
        } else if self.accepted_baseline.is_none() {
            self.accepted_baseline = event.baseline.clone();
        }
        let measurement = event.measurement.config();
        log_perf_start(
            &event.domain,
            &event.subjects,
            &event
                .fixtures
                .iter()
                .map(WasmFixture::fixture)
                .collect::<Vec<_>>(),
            &event
                .scales
                .iter()
                .map(WasmScale::resize_scale)
                .collect::<Vec<_>>(),
            &measurement,
            self.same_run_scalar_comparison.then_some("same-run scalar"),
            self.accepted_baseline.as_deref(),
        );
        self.measurement = Some(measurement);
        Ok(())
    }

    fn logger(&mut self, subject: &str, case: &str) -> Result<&mut MeasurementLogger, BenchError> {
        if self.logger.is_none() {
            self.logger = Some(MeasurementLogger::new(subject, case, self.measurement()?));
        }
        Ok(self.logger.as_mut().expect("logger initialized above"))
    }

    fn measurement(&self) -> Result<MeasurementConfig, BenchError> {
        self.measurement.ok_or_else(|| {
            BenchError::Runtime("browser/Wasm harness emitted measurement before start".to_owned())
        })
    }

    fn accepted_baseline_name(&self) -> Option<&str> {
        self.accepted_baseline.as_deref()
    }

    fn attach_accepted_comparison(
        &self,
        name: &str,
        result: &mut BenchResult,
    ) -> Result<(), BenchError> {
        let run = BenchRun::new(
            "wasm-resize",
            self.domain.as_deref().unwrap_or("resize"),
            Some(self.measurement()?.artifact()),
            vec![result.clone()],
        );
        let accepted_baseline = load_scoped_baseline("accepted", name, &run)?;
        attach_accepted_comparisons(std::slice::from_mut(result), Some(&accepted_baseline));
        Ok(())
    }

    fn attach_same_run_scalar_comparison(&self, result: &mut BenchResult) {
        let Some(baseline_subject) =
            same_run_scalar_baseline_subject(self.profile.as_deref(), result)
        else {
            return;
        };
        if result.subject == baseline_subject {
            return;
        }
        let Some(baseline) = self.results.iter().rev().find(|candidate| {
            candidate.subject == baseline_subject && same_case(result, candidate)
        }) else {
            return;
        };
        result.comparisons.insert(
            "oracle".to_owned(),
            comparison("same-run scalar", result.median_ns, baseline.median_ns),
        );
    }

    fn finish(mut self, flags: &WasmBenchFlags) -> Result<(), BenchError> {
        let measurement = self.measurement.ok_or_else(|| {
            BenchError::Runtime("browser/Wasm harness did not emit start event".to_owned())
        })?;
        attach_same_run_scalar_comparisons(self.profile.as_deref(), &mut self.results);
        print_perf_table(&self.results);

        let domain = self.domain.as_deref().unwrap_or("resize");
        let run = BenchRun::new(
            "wasm-resize",
            domain,
            Some(measurement.artifact()),
            self.results,
        );
        let profile = self.profile.unwrap_or_else(|| "ad-hoc".to_owned());
        let save_baseline_name = flags.save_baseline.as_deref().map(|name| {
            if name == "true" {
                profile.as_str()
            } else {
                name
            }
        });
        save_indexed_run(&run, save_baseline_name)?;
        if let Some(name) = save_baseline_name {
            save_scoped_baseline("accepted", name, &run, true)?;
        }

        let output_dir = output_dir(&flags.harness_args, &run.run_id);
        fs::create_dir_all(&output_dir).map_err(BenchError::io)?;
        let path = output_dir.join(format!("{}.json", sanitize_path_component(&profile)));
        let json = serde_json::to_string_pretty(&run).map_err(|error| {
            BenchError::Runtime(format!(
                "failed to serialize Wasm benchmark artifact: {error}"
            ))
        })?;
        fs::write(&path, format!("{json}\n")).map_err(BenchError::io)?;
        if let Some(name) = save_baseline_name {
            let action = if flags.replacing_baseline {
                "replaced"
            } else {
                "saved"
            };
            println!("{action} accepted baseline {name:?}");
        }
        println!("\nWrote {}", path.display());
        Ok(())
    }
}

fn same_run_scalar_comparison_profile(profile: Option<&str>) -> bool {
    matches!(
        profile,
        Some("nearest-thread" | "convolution-thread" | "convolution-plan-scope" | "color")
    )
}

fn same_run_scalar_baseline_subject(profile: Option<&str>, result: &BenchResult) -> Option<String> {
    match profile? {
        "nearest-thread" => Some("wasm:resize:nearest:scalar".to_owned()),
        "convolution-thread" | "convolution-plan-scope" => {
            Some("wasm:resize:lanczos3:fixed".to_owned())
        }
        "color" => result
            .subject
            .rsplit_once(':')
            .and_then(|(prefix, variant)| {
                (variant != "scalar").then(|| format!("{prefix}:scalar"))
            }),
        _ => None,
    }
}

fn attach_same_run_scalar_comparisons(profile: Option<&str>, results: &mut [BenchResult]) {
    if !same_run_scalar_comparison_profile(profile) {
        return;
    }
    for index in 0..results.len() {
        let Some(baseline_subject) = same_run_scalar_baseline_subject(profile, &results[index])
        else {
            continue;
        };
        if results[index].subject == baseline_subject {
            continue;
        }
        let Some(baseline) = results[..index].iter().rev().find(|candidate| {
            candidate.subject == baseline_subject && same_case(&results[index], candidate)
        }) else {
            continue;
        };
        results[index].comparisons.insert(
            "oracle".to_owned(),
            comparison(
                "same-run scalar",
                results[index].median_ns,
                baseline.median_ns,
            ),
        );
    }
}

fn same_case(result: &BenchResult, baseline: &BenchResult) -> bool {
    same_case_id(result, baseline)
        && result.fixture_fingerprint == baseline.fixture_fingerprint
        && result.source_width == baseline.source_width
        && result.source_height == baseline.source_height
        && result.output_width == baseline.output_width
        && result.output_height == baseline.output_height
        && result_scale_x(result) == result_scale_x(baseline)
        && result_scale_y(result) == result_scale_y(baseline)
        && result.filter == baseline.filter
        && result.pixel_format == baseline.pixel_format
        && result.params_fingerprint == baseline.params_fingerprint
}

fn same_case_id(result: &BenchResult, baseline: &BenchResult) -> bool {
    if result.case_id == baseline.case_id {
        return true;
    }

    result.subject.starts_with("wasm:") && baseline.subject.starts_with("wasm:")
}

fn result_scale_x(result: &BenchResult) -> f64 {
    if result.scale_x == 0.0 {
        result.scale
    } else {
        result.scale_x
    }
}

fn result_scale_y(result: &BenchResult) -> f64 {
    if result.scale_y == 0.0 {
        result.scale
    } else {
        result.scale_y
    }
}

fn comparison(baseline: &str, current_ns: f64, baseline_ns: f64) -> ComparisonReport {
    let ratio = current_ns / baseline_ns;
    ComparisonReport {
        baseline: baseline.to_owned(),
        median_ns: baseline_ns,
        ratio,
        status: if ratio < 0.98 {
            "faster".to_owned()
        } else if ratio > 1.02 {
            "slower".to_owned()
        } else {
            "same".to_owned()
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum WasmEvent {
    Start(StartEvent),
    WarmupBatch(ProgressEvent),
    WarmupFinished(ProgressEvent),
    MeasurementProgress(MeasurementEvent),
    Result(ResultEvent),
    Complete(CompleteEvent),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartEvent {
    #[serde(default = "default_resize_domain")]
    domain: String,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    baseline: Option<String>,
    subjects: Vec<String>,
    scales: Vec<WasmScale>,
    measurement: WasmMeasurement,
    fixtures: Vec<WasmFixture>,
}

fn default_resize_domain() -> String {
    "resize".to_owned()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    subject: String,
    case_id: String,
    batch_size: usize,
    #[serde(default)]
    batch_elapsed_ms: Option<f64>,
    elapsed_ms: f64,
}

impl ProgressEvent {
    fn batch_duration(&self) -> Duration {
        duration_ms(self.batch_elapsed_ms.unwrap_or(self.elapsed_ms))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeasurementEvent {
    subject: String,
    case_id: String,
    samples_done: usize,
    sample_size: usize,
    elapsed_ms: f64,
    measurement_time_ms: f64,
    total_iterations: usize,
    samples_ns: Vec<f64>,
    output: WasmOutput,
}

#[derive(Debug, Deserialize)]
struct ResultEvent {
    result: WasmResult,
}

#[derive(Debug, Deserialize)]
struct CompleteEvent {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmMeasurement {
    sample_size: usize,
    measurement_time_ms: f64,
    warm_up_time_ms: f64,
    warm_up_iterations: usize,
    target_sample_time_ms: f64,
    #[serde(default)]
    live_stats: bool,
}

impl WasmMeasurement {
    fn config(&self) -> MeasurementConfig {
        MeasurementConfig::browser_wasm(
            self.sample_size,
            duration_ms(self.measurement_time_ms),
            (self.warm_up_iterations > 0).then_some(self.warm_up_iterations),
            duration_ms(self.warm_up_time_ms),
            duration_ms(self.target_sample_time_ms),
            self.live_stats,
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WasmScale {
    Uniform(f64),
    Pair { x: f64, y: f64 },
}

impl WasmScale {
    fn resize_scale(&self) -> ResizeScale {
        match *self {
            Self::Uniform(scale) => ResizeScale::uniform(scale),
            Self::Pair { x, y } => ResizeScale { x, y },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmFixture {
    name: String,
    width: u32,
    height: u32,
    fingerprint: String,
    #[serde(default = "browser_image_kind")]
    kind: String,
}

fn browser_image_kind() -> String {
    "browser-image".to_owned()
}

impl WasmFixture {
    fn fixture(&self) -> Fixture {
        Fixture {
            id: self.name.clone(),
            kind: self.kind.clone(),
            fingerprint: self.fingerprint.clone(),
            width: self.width,
            height: self.height,
            rgba: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmResult {
    id: String,
    subject: String,
    filter: String,
    #[serde(default)]
    support_policy: String,
    fixture: WasmResultFixture,
    scale: WasmScale,
    source: WasmOutput,
    output: WasmOutput,
    checksum: u32,
    #[serde(rename = "statsNs")]
    stats_ns: WasmStats,
    iterations_per_sample: usize,
    total_iterations: usize,
}

impl WasmResult {
    fn into_bench_result(self) -> BenchResult {
        let stats = SampleStats::from_samples(&self.stats_ns.samples);
        let scale = self.scale.resize_scale();
        let median_seconds = stats.median_ns / 1_000_000_000.0;
        let output_pixels = f64::from(self.output.width) * f64::from(self.output.height);
        let fixture_name = self.fixture.name;
        BenchResult {
            subject: self.subject.clone(),
            case_id: self.id,
            fixture_fingerprint: self.fixture.fingerprint,
            fixture: fixture_name,
            fixture_kind: self.fixture.kind.unwrap_or_else(browser_image_kind),
            filter: self.filter,
            variant: wasm_variant(&self.subject, &self.support_policy),
            source_width: self.source.width,
            source_height: self.source.height,
            output_width: self.output.width,
            output_height: self.output.height,
            scale: scale.x,
            scale_x: scale.x,
            scale_y: scale.y,
            pixel_format: "rgba8".to_owned(),
            params_fingerprint: "wasm-resize-default".to_owned(),
            verified: false,
            verification: None,
            checksum: format!("{:08x}", self.checksum),
            samples: self.stats_ns.samples.len(),
            sample_ns: self.stats_ns.samples,
            iterations_per_sample: self.iterations_per_sample,
            total_iterations: self.total_iterations,
            min_ns: stats.min_ns,
            median_ns: stats.median_ns,
            mean_ns: stats.mean_ns,
            stdev_ns: stats.stdev_ns,
            mode_ns: stats.mode_ns,
            p75_ns: stats.p75_ns,
            p90_ns: stats.p90_ns,
            p95_ns: stats.p95_ns,
            p99_ns: stats.p99_ns,
            max_ns: stats.max_ns,
            output_mpix_per_s: output_pixels / median_seconds / 1_000_000.0,
            comparisons: BTreeMap::new(),
        }
    }
}

fn wasm_variant(subject: &str, support_policy: &str) -> String {
    subject
        .rsplit(':')
        .next()
        .filter(|variant| !variant.is_empty())
        .unwrap_or(support_policy)
        .to_owned()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmResultFixture {
    name: String,
    fingerprint: String,
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WasmOutput {
    width: u32,
    height: u32,
}

#[derive(Debug, Deserialize)]
struct WasmStats {
    samples: Vec<f64>,
}

fn duration_ms(ms: f64) -> Duration {
    Duration::from_secs_f64((ms / 1_000.0).max(0.0))
}

fn output_dir(args: &[String], run_id: &str) -> PathBuf {
    for (index, arg) in args.iter().enumerate() {
        if let Some(value) = arg
            .strip_prefix("--output-dir=")
            .or_else(|| arg.strip_prefix("--out="))
        {
            return PathBuf::from(value);
        }
        if matches!(arg.as_str(), "--output-dir" | "--out") {
            if let Some(value) = args.get(index + 1) {
                return PathBuf::from(value);
            }
        }
    }
    Path::new("benchmark-results").join(format!("wasm-resize-{run_id}"))
}

fn sanitize_path_component(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|char| match char {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '.' | '-' => char,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if sanitized.is_empty() {
        "run".to_owned()
    } else {
        sanitized
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn invalid_jsonl_stops_and_reaps_the_transport() {
        let lease = Lease::exclusive().unwrap();
        let marker =
            std::env::temp_dir().join(format!("ditherette-jsonl-cleanup-{}", std::process::id()));
        let mut command = Command::new("node");
        command.args(["-e", "process.on('SIGTERM', () => { require('node:fs').writeFileSync(process.env.DITHERETTE_CLEANUP_MARKER, 'closed'); process.exit(0); }); console.log('{bad json'); setInterval(() => {}, 1000);"])
            .env("DITHERETTE_CLEANUP_MARKER", &marker).stdout(Stdio::piped());
        let error = run_harness(&lease, command, &WasmBenchFlags::parse(&[]).unwrap()).unwrap_err();
        assert!(error.to_string().contains("invalid JSONL"));
        assert_eq!(fs::read_to_string(&marker).unwrap(), "closed");
        fs::remove_file(marker).unwrap();
    }
}

#[cfg(test)]
mod benchmark_config_tests {
    use super::*;
    use serde_json::json;

    fn result(subject: &str, fingerprint: &str, sample: f64) -> BenchResult {
        serde_json::from_value::<WasmResult>(json!({
            "id": "fixture-decoded-rgba",
            "subject": subject,
            "filter": "lanczos3",
            "fixture": { "name": "fixture.png", "fingerprint": fingerprint },
            "scale": 1.0,
            "source": { "width": 1, "height": 1 },
            "output": { "width": 1, "height": 1 },
            "checksum": 0,
            "statsNs": { "samples": [sample] },
            "iterationsPerSample": 1,
            "totalIterations": 1
        }))
        .unwrap()
        .into_bench_result()
    }

    #[test]
    fn decoded_fingerprints_reach_results_and_fixture_metadata() {
        let fingerprint = "rgba8:1x1:fnv1a32:12345678";
        let fixture: WasmFixture = serde_json::from_value(json!({
            "name": "fixture.png", "width": 1, "height": 1, "fingerprint": fingerprint
        }))
        .unwrap();
        assert_eq!(fixture.fixture().fingerprint, fingerprint);
        let left = result("wasm:resize:lanczos3:fixed", fingerprint, 100.0);
        assert_eq!(left.fixture_fingerprint, fingerprint);
        let right = result(
            "wasm:resize:lanczos3:fixed",
            "rgba8:1x1:fnv1a32:87654321",
            100.0,
        );
        assert!(!same_case(&left, &right));
    }

    #[test]
    fn missing_decoded_fingerprints_are_rejected() {
        assert!(serde_json::from_value::<WasmFixture>(json!({
            "name": "fixture.png", "width": 1, "height": 1
        }))
        .is_err());
        assert!(
            serde_json::from_value::<WasmResultFixture>(json!({ "name": "fixture.png" })).is_err()
        );
    }

    #[test]
    fn periodic_comparisons_use_the_latest_preceding_scalar() {
        let scalar = "wasm:resize:lanczos3:fixed";
        let candidate = "wasm:resize:lanczos3:pooled_direct";
        let fingerprint = "rgba8:1x1:fnv1a32:12345678";
        let mut results = vec![
            result(scalar, fingerprint, 100.0),
            result(candidate, fingerprint, 50.0),
            result(scalar, fingerprint, 200.0),
            result(candidate, fingerprint, 50.0),
        ];
        attach_same_run_scalar_comparisons(Some("convolution-thread"), &mut results);
        assert_eq!(results[1].comparisons["oracle"].median_ns, 100.0);
        assert_eq!(results[3].comparisons["oracle"].median_ns, 200.0);
        let state = WasmRunState {
            profile: Some("convolution-thread".into()),
            results: results[..3].to_vec(),
            ..WasmRunState::default()
        };
        let mut current = result(candidate, fingerprint, 50.0);
        state.attach_same_run_scalar_comparison(&mut current);
        assert_eq!(current.comparisons["oracle"].median_ns, 200.0);
    }
}
