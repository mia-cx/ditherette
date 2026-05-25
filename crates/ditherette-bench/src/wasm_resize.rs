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

use serde::Deserialize;

use crate::{
    baseline::{load_scoped_baseline, save_indexed_run, save_scoped_baseline},
    case::ResizeScale,
    compare::attach_accepted_comparisons,
    error::BenchError,
    fixture::Fixture,
    measure::{MeasurementConfig, MeasurementObserver, MeasurementProgress},
    report::{log_perf_start, print_perf_table, MeasurementLogger},
    result::{BenchResult, BenchRun, SampleStats},
};

const WASM_MANIFEST: &str = "scripts/ditherette-wasm-bench.toml";
const WASM_HARNESS: &str = "scripts/benchmark-wasm-resize.mjs";

pub(crate) fn wasm_resize_command(args: &[String]) -> Result<(), BenchError> {
    let args = strip_leading_separator(args);
    let bench_flags = WasmBenchFlags::parse(args)?;
    let mut child = Command::new("node")
        .arg(WASM_HARNESS)
        .args(with_default_config(&bench_flags.harness_args))
        .arg("--jsonl-events")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| {
            BenchError::Runtime(format!("failed to start browser/Wasm harness: {error}"))
        })?;

    let stdout = child.stdout.take().ok_or_else(|| {
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
}

impl WasmBenchFlags {
    fn parse(args: &[String]) -> Result<Self, BenchError> {
        let mut harness_args = Vec::new();
        let mut accepted_baseline = None;
        let mut save_baseline = None;
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if let Some(value) = arg.strip_prefix("--baseline=") {
                accepted_baseline = Some(value.to_owned());
                index += 1;
            } else if arg == "--baseline" {
                let value = args.get(index + 1).ok_or_else(|| {
                    BenchError::Config("--baseline requires a baseline name".to_owned())
                })?;
                accepted_baseline = Some(value.clone());
                index += 2;
            } else if let Some(value) = arg.strip_prefix("--save-baseline=") {
                save_baseline = Some(value.to_owned());
                index += 1;
            } else if arg == "--save-baseline" {
                if args
                    .get(index + 1)
                    .is_some_and(|next| !next.starts_with("--"))
                {
                    save_baseline = Some(args[index + 1].clone());
                    index += 2;
                } else {
                    save_baseline = Some("true".to_owned());
                    index += 1;
                }
            } else {
                harness_args.push(arg.clone());
                index += 1;
            }
        }
        Ok(Self {
            harness_args,
            accepted_baseline,
            save_baseline,
        })
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
}

impl WasmRunState {
    fn handle(&mut self, event: WasmEvent) -> Result<(), BenchError> {
        match event {
            WasmEvent::Start(event) => self.start(event),
            WasmEvent::WarmupBatch(event) => {
                self.logger(&event.subject, &event.case_id)?
                    .warmup_batch(event.batch_size, duration_ms(event.elapsed_ms));
                Ok(())
            }
            WasmEvent::WarmupFinished(event) => {
                self.logger(&event.subject, &event.case_id)?
                    .warmup_finished(event.batch_size, duration_ms(event.elapsed_ms));
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
                if let Some(name) = self.accepted_baseline_name() {
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
        self.profile = event.profile;
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
            None,
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
            "resize",
            Some(self.measurement()?.artifact()),
            vec![result.clone()],
        );
        let accepted_baseline = load_scoped_baseline("accepted", name, &run)?;
        attach_accepted_comparisons(std::slice::from_mut(result), Some(&accepted_baseline));
        Ok(())
    }

    fn finish(self, flags: &WasmBenchFlags) -> Result<(), BenchError> {
        let measurement = self.measurement.ok_or_else(|| {
            BenchError::Runtime("browser/Wasm harness did not emit start event".to_owned())
        })?;
        print_perf_table(&self.results);

        let run = BenchRun::new(
            "wasm-resize",
            "resize",
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
            println!("saved accepted baseline {name:?}");
        }
        println!("\nWrote {}", path.display());
        Ok(())
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
    elapsed_ms: f64,
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
            fingerprint: format!("browser:{}:{}x{}", self.name, self.width, self.height),
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
            fixture_fingerprint: format!(
                "browser:{}:{}x{}",
                fixture_name, self.source.width, self.source.height
            ),
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
