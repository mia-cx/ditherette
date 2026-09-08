//! Timing loops for resize benchmark subjects.

use std::{
    collections::BTreeMap,
    hint::black_box,
    thread,
    time::{Duration, Instant},
};

use ditherette_bench_api::{
    ResizeBenchSubject, ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams,
};

use crate::{
    case::ResizeScale,
    cli::Flags,
    error::BenchError,
    fixture::Fixture,
    result::{BenchResult, MeasurementArtifact, SampleStats, VerificationReport},
    runtime::ProcessPriority,
    util::{checksum, RGBA_CHANNELS},
};

const DEFAULT_SAMPLE_SIZE: usize = 100;
const DEFAULT_MEASUREMENT_SECONDS: u64 = 5;
const DEFAULT_WARMUP_SECONDS: u64 = 1;
const MIN_AUTO_TARGET_SAMPLE: Duration = Duration::from_millis(1);
const DEFAULT_CACHE_SCRUB_BYTES: usize = 64 * 1024 * 1024;
const CACHE_LINE_STRIDE: usize = 64;
const MAX_CALIBRATION_BATCH: usize = 1 << 30;
const WARMUP_DISCARD_ITERATIONS: usize = 2;
const MIN_BATCH_REFINEMENTS: usize = 3;

pub(crate) trait MeasurementObserver {
    fn warmup_batch(&mut self, _batch_size: usize, _elapsed: Duration) {}
    fn warmup_finished(&mut self, _batch_size: usize, _elapsed: Duration) {}

    /// Returns true when reporting did enough work that the next timed batch should be discarded.
    fn measurement_progress(
        &mut self,
        _progress: MeasurementProgress,
        _samples: &[f64],
        _output: (u32, u32),
    ) -> bool {
        false
    }
}

impl MeasurementObserver for () {}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MeasurementProgress {
    pub(crate) samples_done: usize,
    pub(crate) sample_size: usize,
    pub(crate) elapsed: Duration,
    pub(crate) measurement_time: Duration,
    pub(crate) total_iterations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SampleMode {
    Throughput,
    Interactive,
}

impl SampleMode {
    fn parse(value: &str) -> Result<Self, BenchError> {
        match value {
            "throughput" => Ok(Self::Throughput),
            "interactive" => Ok(Self::Interactive),
            _ => Err(BenchError::Config(format!(
                "invalid --sample-mode {value:?}; expected throughput or interactive"
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Throughput => "throughput",
            Self::Interactive => "interactive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheState {
    Warm,
    Scrubbed,
}

impl CacheState {
    fn parse(value: &str) -> Result<Self, BenchError> {
        match value {
            "warm" => Ok(Self::Warm),
            "scrubbed" => Ok(Self::Scrubbed),
            _ => Err(BenchError::Config(format!(
                "invalid --cache-state {value:?}; expected warm or scrubbed"
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::Scrubbed => "scrubbed",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MeasurementConfig {
    sample_size: usize,
    measurement_time: Duration,
    warmup_iterations: Option<usize>,
    warmup_time: Duration,
    target_sample: Duration,
    sample_mode: SampleMode,
    cache_state: CacheState,
    cache_scrub_size: usize,
    inter_sample_delay: Duration,
    live_stats: bool,
    preheat_time: Duration,
    process_priority: ProcessPriority,
}

impl MeasurementConfig {
    pub(crate) fn sample_size(&self) -> usize {
        self.sample_size
    }

    pub(crate) fn measurement_time(&self) -> Duration {
        self.measurement_time
    }

    pub(crate) fn warmup_time(&self) -> Duration {
        self.warmup_time
    }

    pub(crate) fn warmup_iterations(&self) -> Option<usize> {
        self.warmup_iterations
    }

    pub(crate) fn target_sample(&self) -> Duration {
        self.target_sample
    }

    pub(crate) fn sample_mode(&self) -> SampleMode {
        self.sample_mode
    }

    pub(crate) fn cache_state(&self) -> CacheState {
        self.cache_state
    }

    pub(crate) fn cache_scrub_size(&self) -> usize {
        self.cache_scrub_size
    }

    pub(crate) fn inter_sample_delay(&self) -> Duration {
        self.inter_sample_delay
    }

    pub(crate) fn live_stats(&self) -> bool {
        self.live_stats
    }

    pub(crate) fn preheat_time(&self) -> Duration {
        self.preheat_time
    }

    pub(crate) fn process_priority(&self) -> ProcessPriority {
        self.process_priority
    }

    pub(crate) fn browser_wasm(
        sample_size: usize,
        measurement_time: Duration,
        warmup_iterations: Option<usize>,
        warmup_time: Duration,
        target_sample: Duration,
        live_stats: bool,
    ) -> Self {
        Self {
            sample_size,
            measurement_time,
            warmup_iterations,
            warmup_time,
            target_sample,
            sample_mode: SampleMode::Throughput,
            cache_state: CacheState::Warm,
            cache_scrub_size: 0,
            inter_sample_delay: Duration::ZERO,
            live_stats,
            preheat_time: Duration::ZERO,
            process_priority: ProcessPriority::Normal,
        }
    }

    pub(crate) fn artifact(&self) -> MeasurementArtifact {
        MeasurementArtifact {
            sample_size: self.sample_size,
            measurement_time_ms: self.measurement_time.as_millis(),
            warmup_iterations: self.warmup_iterations,
            warmup_time_ms: self.warmup_time.as_millis(),
            target_sample_time_ms: self.target_sample.as_millis(),
            sample_mode: self.sample_mode.as_str().to_owned(),
            cache_state: self.cache_state.as_str().to_owned(),
            cache_scrub_size: self.cache_scrub_size,
            inter_sample_delay_ms: self.inter_sample_delay.as_millis(),
            live_stats: self.live_stats,
            preheat_time_ms: self.preheat_time.as_millis(),
            process_priority: self.process_priority.as_str().to_owned(),
        }
    }
}

impl MeasurementConfig {
    pub(crate) fn from_flags(flags: &Flags) -> Result<Self, BenchError> {
        let sample_size = flags
            .optional("--sample-size")
            .or_else(|| flags.optional("--samples"))
            .or_else(|| flags.optional("--iterations"))
            .or_else(|| flags.optional("--measurement-iterations"))
            .map(parse_usize_flag)
            .transpose()?
            .unwrap_or(DEFAULT_SAMPLE_SIZE);
        if sample_size == 0 {
            return Err(BenchError::Config(
                "--sample-size must be greater than zero".to_owned(),
            ));
        }

        let warmup_iterations = flags
            .optional("--warmup-iterations")
            .or_else(|| flags.optional("--warm-up-iterations"))
            .map(parse_usize_flag)
            .transpose()?;

        let sample_mode = flags
            .optional("--sample-mode")
            .map(SampleMode::parse)
            .transpose()?
            .unwrap_or(SampleMode::Throughput);
        let cache_state = flags
            .optional("--cache-state")
            .map(CacheState::parse)
            .transpose()?
            .unwrap_or(CacheState::Warm);
        let cache_scrub_size = flags
            .optional("--cache-scrub-size")
            .map(parse_byte_size_flag)
            .transpose()?
            .unwrap_or(DEFAULT_CACHE_SCRUB_BYTES);
        let inter_sample_delay = duration_from_flags(
            flags,
            &[
                ("--inter-sample-delay", DurationUnit::Millis),
                ("--inter-sample-delay-ms", DurationUnit::Millis),
            ],
        )?
        .unwrap_or(Duration::ZERO);
        let measurement_time = duration_from_flags(
            flags,
            &[
                ("--measurement-time", DurationUnit::Seconds),
                ("--measurement-time-ms", DurationUnit::Millis),
            ],
        )?
        .unwrap_or_else(|| Duration::from_secs(DEFAULT_MEASUREMENT_SECONDS));

        let preheat_time = duration_from_flags(
            flags,
            &[
                ("--preheat-time", DurationUnit::Seconds),
                ("--preheat-time-ms", DurationUnit::Millis),
            ],
        )?
        .unwrap_or(Duration::ZERO);
        let process_priority = flags
            .optional("--process-priority")
            .map(ProcessPriority::parse)
            .transpose()?
            .unwrap_or(ProcessPriority::Normal);

        let config = Self {
            sample_size,
            measurement_time,
            warmup_iterations,
            warmup_time: duration_from_flags(
                flags,
                &[
                    ("--warm-up-time", DurationUnit::Seconds),
                    ("--warmup-time", DurationUnit::Seconds),
                    ("--warmup-ms", DurationUnit::Millis),
                ],
            )?
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_WARMUP_SECONDS)),
            target_sample: target_sample_from_flags(flags, sample_size, measurement_time)?,
            sample_mode,
            cache_state,
            cache_scrub_size,
            inter_sample_delay,
            live_stats: flags
                .optional("--live-stats")
                .map(parse_bool_flag)
                .transpose()?
                .unwrap_or(false),
            preheat_time,
            process_priority,
        };

        if config.measurement_time.is_zero() {
            return Err(BenchError::Config(
                "--measurement-time must be greater than zero".to_owned(),
            ));
        }
        if config.warmup_time.is_zero() && config.warmup_iterations == Some(0) {
            return Err(BenchError::Config(
                "warmup must have a non-zero time or iteration target".to_owned(),
            ));
        }
        if config.target_sample.is_zero() {
            return Err(BenchError::Config(
                "--target-sample-time must be greater than zero".to_owned(),
            ));
        }
        if config.cache_state == CacheState::Scrubbed && config.cache_scrub_size == 0 {
            return Err(BenchError::Config(
                "--cache-scrub-size must be greater than zero with --cache-state scrubbed"
                    .to_owned(),
            ));
        }

        Ok(config)
    }
}

#[derive(Debug, Clone, Copy)]
enum DurationUnit {
    Seconds,
    Millis,
}

pub(crate) fn measure_resize_case(
    subject: &ResizeBenchSubject,
    fixture: &Fixture,
    output: (u32, u32),
    scale: ResizeScale,
    params: &ResizeParams,
    config: &MeasurementConfig,
    verification: Option<VerificationReport>,
    observer: &mut impl MeasurementObserver,
) -> Result<BenchResult, BenchError> {
    let mut workload = ResizeWorkload {
        subject,
        fixture,
        output,
        params,
        rgba: vec![0; output.0 as usize * output.1 as usize * RGBA_CHANNELS],
    };
    let measured = measure_workload(&mut workload, output, config, observer)?;
    let sample_ns = measured.sample_ns;
    let iterations_per_sample = measured.iterations_per_sample;
    let measured_iterations = measured.total_iterations;
    let final_checksum = checksum(&workload.rgba);

    let stats = SampleStats::from_samples(&sample_ns);
    let output_pixels = f64::from(output.0) * f64::from(output.1);
    let median_seconds = stats.median_ns / 1_000_000_000.0;

    Ok(BenchResult {
        subject: subject.descriptor.id.to_string(),
        case_id: format!("{}-{}x{}-{}", fixture.id, output.0, output.1, scale.label()),
        fixture: fixture.id.clone(),
        fixture_kind: fixture.kind.clone(),
        fixture_fingerprint: fixture.fingerprint.clone(),
        filter: subject.descriptor.id.filter().to_owned(),
        variant: subject.descriptor.id.variant().to_owned(),
        source_width: fixture.width,
        source_height: fixture.height,
        output_width: output.0,
        output_height: output.1,
        scale: scale.x,
        scale_x: scale.x,
        scale_y: scale.y,
        pixel_format: "rgba8".to_owned(),
        params_fingerprint: "resize-default".to_owned(),
        verified: verification
            .as_ref()
            .is_some_and(|verification| verification.passed),
        verification,
        checksum: final_checksum,
        samples: sample_ns.len(),
        sample_ns,
        iterations_per_sample,
        total_iterations: measured_iterations,
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
    })
}

/// One iteration's owned work. Consumption runs outside every timed batch.
pub(crate) trait Workload {
    /// Interactive sample setup, outside its timer. Throughput keeps its existing batch lifecycle.
    fn prepare_sample(&mut self) -> Result<(), BenchError> {
        Ok(())
    }
    fn run(&mut self) -> Result<(), BenchError>;
    fn consume(&self);
    /// Runs after observation, including when the call fails.
    fn finish_sample(&mut self) {}
}

fn interactive_sample(workload: &mut impl Workload) -> Result<Duration, BenchError> {
    if let Err(error) = workload.prepare_sample() {
        workload.finish_sample();
        return Err(error);
    }
    let start = Instant::now();
    let result = workload.run();
    let elapsed = start.elapsed();
    workload.consume();
    workload.finish_sample();
    result.map(|()| elapsed)
}

pub(crate) struct MeasuredSamples {
    pub sample_ns: Vec<f64>,
    pub iterations_per_sample: usize,
    pub total_iterations: usize,
}

struct ResizeWorkload<'a> {
    subject: &'a ResizeBenchSubject,
    fixture: &'a Fixture,
    output: (u32, u32),
    params: &'a ResizeParams,
    rgba: Vec<u8>,
}

impl Workload for ResizeWorkload<'_> {
    fn run(&mut self) -> Result<(), BenchError> {
        run_resize_into(
            self.subject,
            self.fixture,
            self.output,
            &mut self.rgba,
            self.params,
        )
    }
    fn consume(&self) {
        black_box(checksum(&self.rgba));
    }
}

/// Shared resize, conversion, and complete native call loop.
pub(crate) fn measure_workload(
    workload: &mut impl Workload,
    output: (u32, u32),
    config: &MeasurementConfig,
    observer: &mut impl MeasurementObserver,
) -> Result<MeasuredSamples, BenchError> {
    let iterations_per_sample = match config.sample_mode {
        SampleMode::Throughput => warm_up_and_calibrate(workload, config, observer)?,
        SampleMode::Interactive => {
            warm_up_interactive(workload, config, observer)?;
            1
        }
    };
    let mut cache_scrubber = CacheScrubber::new(config.cache_state, config.cache_scrub_size);
    let mut sample_ns = Vec::new();
    let mut measured_iterations = 0usize;
    let mut measured_elapsed = Duration::ZERO;
    let mut discard_next_batch = observer.measurement_progress(
        MeasurementProgress {
            samples_done: 0,
            sample_size: config.sample_size,
            elapsed: Duration::ZERO,
            measurement_time: config.measurement_time,
            total_iterations: 0,
        },
        &sample_ns,
        output,
    );

    while sample_ns.is_empty()
        || (sample_ns.len() < config.sample_size && measured_elapsed < config.measurement_time)
    {
        let batch_iterations = iterations_per_sample;
        if std::mem::take(&mut discard_next_batch) {
            cache_scrubber.prepare();
            if config.sample_mode == SampleMode::Interactive {
                interactive_sample(workload)?;
            } else {
                for _ in 0..batch_iterations {
                    workload.run()?;
                }
            }
        }
        if !config.inter_sample_delay.is_zero() {
            thread::sleep(config.inter_sample_delay);
        }
        cache_scrubber.prepare();

        let elapsed = if config.sample_mode == SampleMode::Interactive {
            interactive_sample(workload)?
        } else {
            let start = Instant::now();
            for _ in 0..batch_iterations {
                workload.run()?;
            }
            start.elapsed()
        };
        measured_iterations += batch_iterations;
        measured_elapsed += elapsed;
        sample_ns.push(elapsed.as_nanos() as f64 / batch_iterations as f64);
        discard_next_batch = observer.measurement_progress(
            MeasurementProgress {
                samples_done: sample_ns.len(),
                sample_size: config.sample_size,
                elapsed: measured_elapsed,
                measurement_time: config.measurement_time,
                total_iterations: measured_iterations,
            },
            &sample_ns,
            output,
        );
    }

    workload.consume();
    Ok(MeasuredSamples {
        sample_ns,
        iterations_per_sample,
        total_iterations: measured_iterations,
    })
}

pub(crate) fn run_resize_once(
    subject: &ResizeBenchSubject,
    fixture: &Fixture,
    output: (u32, u32),
    params: &ResizeParams,
) -> Result<Vec<u8>, BenchError> {
    let mut output_rgba = vec![0; output.0 as usize * output.1 as usize * RGBA_CHANNELS];
    run_resize_into(subject, fixture, output, &mut output_rgba, params)?;
    Ok(output_rgba)
}

fn warm_up_interactive(
    workload: &mut impl Workload,
    config: &MeasurementConfig,
    observer: &mut impl MeasurementObserver,
) -> Result<(), BenchError> {
    let warmup_start = Instant::now();
    let mut warmup_iterations = 0usize;
    loop {
        let hit_iteration_target = config
            .warmup_iterations
            .is_some_and(|target| warmup_iterations >= target);
        let hit_time_target = warmup_start.elapsed() >= config.warmup_time;
        if warmup_iterations > 0 && (hit_time_target || hit_iteration_target) {
            return Ok(());
        }

        let elapsed = interactive_sample(workload)?;
        warmup_iterations += 1;
        observer.warmup_batch(1, elapsed);

        let hit_iteration_target = config
            .warmup_iterations
            .is_some_and(|target| warmup_iterations >= target);
        let hit_time_target = warmup_start.elapsed() >= config.warmup_time;
        if hit_time_target || hit_iteration_target {
            observer.warmup_finished(1, elapsed);
            return Ok(());
        }
    }
}

fn warm_up_and_calibrate(
    workload: &mut impl Workload,
    config: &MeasurementConfig,
    observer: &mut impl MeasurementObserver,
) -> Result<usize, BenchError> {
    let warmup_start = Instant::now();
    let mut warmup_iterations = 0usize;

    let discard_start = Instant::now();
    for _ in 0..WARMUP_DISCARD_ITERATIONS {
        workload.run()?;
        warmup_iterations += 1;
    }
    observer.warmup_batch(WARMUP_DISCARD_ITERATIONS, discard_start.elapsed());
    workload.consume();

    let single_start = Instant::now();
    workload.run()?;
    let single_elapsed = single_start.elapsed();
    warmup_iterations += 1;
    observer.warmup_batch(1, single_elapsed);
    workload.consume();

    if single_elapsed >= config.target_sample {
        continue_warmup(
            workload,
            config,
            observer,
            warmup_start,
            warmup_iterations,
            1,
        )?;
        observer.warmup_finished(1, single_elapsed);
        return Ok(1);
    }

    let mut best_batch_iterations = 1;
    let mut best_batch_elapsed = single_elapsed;
    let mut best_target_distance = duration_distance(single_elapsed, config.target_sample);
    let mut batch_iterations = estimate_batch_size(config.target_sample, single_elapsed)?;
    let mut refinements = 0usize;

    loop {
        let start = Instant::now();
        for _ in 0..batch_iterations {
            workload.run()?;
        }
        let elapsed = start.elapsed();
        warmup_iterations += batch_iterations;
        refinements += 1;
        observer.warmup_batch(batch_iterations, elapsed);
        workload.consume();

        let hit_iteration_target = config
            .warmup_iterations
            .is_some_and(|target| warmup_iterations >= target);
        let hit_time_target = warmup_start.elapsed() >= config.warmup_time;
        let target_distance = duration_distance(elapsed, config.target_sample);
        if target_distance < best_target_distance {
            best_target_distance = target_distance;
            best_batch_iterations = batch_iterations;
            best_batch_elapsed = elapsed;
        }

        if refinements >= MIN_BATCH_REFINEMENTS && (hit_time_target || hit_iteration_target) {
            observer.warmup_finished(best_batch_iterations, best_batch_elapsed);
            return Ok(best_batch_iterations);
        }

        batch_iterations = next_calibration_batch(config.target_sample, batch_iterations, elapsed)?;
    }
}

fn continue_warmup(
    workload: &mut impl Workload,
    config: &MeasurementConfig,
    observer: &mut impl MeasurementObserver,
    warmup_start: Instant,
    mut warmup_iterations: usize,
    batch_iterations: usize,
) -> Result<(), BenchError> {
    loop {
        let hit_iteration_target = config
            .warmup_iterations
            .is_some_and(|target| warmup_iterations >= target);
        let hit_time_target = warmup_start.elapsed() >= config.warmup_time;
        if hit_time_target || hit_iteration_target {
            return Ok(());
        }

        let start = Instant::now();
        for _ in 0..batch_iterations {
            workload.run()?;
        }
        let elapsed = start.elapsed();
        warmup_iterations += batch_iterations;
        observer.warmup_batch(batch_iterations, elapsed);
        workload.consume();
    }
}

fn next_calibration_batch(
    target_sample: Duration,
    current_batch: usize,
    current_elapsed: Duration,
) -> Result<usize, BenchError> {
    if current_elapsed.is_zero() {
        return Ok(MAX_CALIBRATION_BATCH);
    }
    let target_ns = target_sample.as_nanos();
    let elapsed_ns = current_elapsed.as_nanos().max(1);
    let batch = ((current_batch as u128 * target_ns) + elapsed_ns / 2) / elapsed_ns;
    if batch > MAX_CALIBRATION_BATCH as u128 {
        return Err(BenchError::Runtime(format!(
            "could not calibrate a sample batch to {} before reaching {MAX_CALIBRATION_BATCH} iterations",
            target_sample.as_millis()
        )));
    }
    Ok(batch.max(1) as usize)
}

fn duration_distance(left: Duration, right: Duration) -> Duration {
    if left >= right {
        left - right
    } else {
        right - left
    }
}

fn estimate_batch_size(
    target_sample: Duration,
    per_iteration: Duration,
) -> Result<usize, BenchError> {
    if per_iteration.is_zero() {
        return Ok(MAX_CALIBRATION_BATCH);
    }
    let target_ns = target_sample.as_nanos();
    let iteration_ns = per_iteration.as_nanos().max(1);
    let batch = ((target_ns + iteration_ns / 2) / iteration_ns).max(1);
    if batch > MAX_CALIBRATION_BATCH as u128 {
        return Err(BenchError::Runtime(format!(
            "could not calibrate a sample batch to {} before reaching {MAX_CALIBRATION_BATCH} iterations",
            target_sample.as_millis()
        )));
    }
    Ok(batch as usize)
}

fn run_resize_into(
    subject: &ResizeBenchSubject,
    fixture: &Fixture,
    output: (u32, u32),
    output_rgba: &mut [u8],
    params: &ResizeParams,
) -> Result<(), BenchError> {
    subject
        .resize_u8_rgba(
            ResizeInputU8Rgba {
                data: &fixture.rgba,
                width: fixture.width,
                height: fixture.height,
                row_stride_elements: fixture.width as usize * RGBA_CHANNELS,
            },
            ResizeOutputU8Rgba {
                data: output_rgba,
                width: output.0,
                height: output.1,
                row_stride_elements: output.0 as usize * RGBA_CHANNELS,
            },
            params,
        )
        .map_err(|error| BenchError::Runtime(error.to_string()))
}

struct CacheScrubber {
    data: Vec<u8>,
    salt: u8,
}

impl CacheScrubber {
    fn new(cache_state: CacheState, size: usize) -> Self {
        let data = match cache_state {
            CacheState::Warm => Vec::new(),
            CacheState::Scrubbed => vec![0; size],
        };
        Self { data, salt: 1 }
    }

    fn prepare(&mut self) {
        if self.data.is_empty() {
            return;
        }
        let mut accumulator = self.salt;
        for byte in self.data.iter_mut().step_by(CACHE_LINE_STRIDE) {
            *byte = byte.wrapping_add(accumulator);
            accumulator = accumulator.wrapping_add(*byte | 1);
        }
        self.salt = accumulator;
        black_box(accumulator);
        black_box(&self.data);
    }
}

fn target_sample_from_flags(
    flags: &Flags,
    sample_size: usize,
    measurement_time: Duration,
) -> Result<Duration, BenchError> {
    let auto_target = || (measurement_time / sample_size as u32).max(MIN_AUTO_TARGET_SAMPLE);
    let Some(value) = flags.optional("--target-sample-time") else {
        return Ok(auto_target());
    };
    if value == "auto" {
        return Ok(auto_target());
    }
    parse_duration_flag(value, DurationUnit::Millis)
}

fn duration_from_flags(
    flags: &Flags,
    keys: &[(&str, DurationUnit)],
) -> Result<Option<Duration>, BenchError> {
    for (key, default_unit) in keys {
        if let Some(value) = flags.optional(key) {
            return parse_duration_flag(value, *default_unit).map(Some);
        }
    }
    Ok(None)
}

fn parse_duration_flag(value: &str, default_unit: DurationUnit) -> Result<Duration, BenchError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BenchError::Config("duration cannot be empty".to_owned()));
    }

    if let Some(milliseconds) = trimmed.strip_suffix("ms") {
        let milliseconds = parse_f64_flag(milliseconds)?;
        return Ok(Duration::from_secs_f64(milliseconds / 1_000.0));
    }
    if let Some(seconds) = trimmed.strip_suffix('s') {
        let seconds = parse_f64_flag(seconds)?;
        return Ok(Duration::from_secs_f64(seconds));
    }

    let value = parse_f64_flag(trimmed)?;
    Ok(match default_unit {
        DurationUnit::Seconds => Duration::from_secs_f64(value),
        DurationUnit::Millis => Duration::from_secs_f64(value / 1_000.0),
    })
}

fn parse_usize_flag(value: &str) -> Result<usize, BenchError> {
    value
        .parse()
        .map_err(|error| BenchError::Config(format!("invalid integer {value:?}: {error}")))
}

fn parse_bool_flag(value: &str) -> Result<bool, BenchError> {
    match value {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(BenchError::Config(format!(
            "invalid boolean {value:?}; expected true or false"
        ))),
    }
}

fn parse_byte_size_flag(value: &str) -> Result<usize, BenchError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(BenchError::Config("byte size cannot be empty".to_owned()));
    }
    let (number, multiplier) = if let Some(number) = trimmed.strip_suffix("KiB") {
        (number, 1024.0)
    } else if let Some(number) = trimmed.strip_suffix("MiB") {
        (number, 1024.0 * 1024.0)
    } else if let Some(number) = trimmed.strip_suffix("GiB") {
        (number, 1024.0 * 1024.0 * 1024.0)
    } else if let Some(number) = trimmed.strip_suffix("KB") {
        (number, 1_000.0)
    } else if let Some(number) = trimmed.strip_suffix("MB") {
        (number, 1_000_000.0)
    } else if let Some(number) = trimmed.strip_suffix("GB") {
        (number, 1_000_000_000.0)
    } else if let Some(number) = trimmed.strip_suffix('B') {
        (number, 1.0)
    } else {
        (trimmed, 1.0)
    };
    let bytes = parse_f64_flag(number)? * multiplier;
    if !bytes.is_finite() || bytes < 0.0 || bytes > usize::MAX as f64 {
        return Err(BenchError::Config(format!(
            "invalid byte size {value:?}: out of range"
        )));
    }
    Ok(bytes.round() as usize)
}

fn parse_f64_flag(value: &str) -> Result<f64, BenchError> {
    value
        .parse()
        .map_err(|error| BenchError::Config(format!("invalid duration {value:?}: {error}")))
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::cell::RefCell;

    struct Tracked {
        events: RefCell<Vec<&'static str>>,
        fail: bool,
        fail_prepare: bool,
    }
    impl Workload for Tracked {
        fn prepare_sample(&mut self) -> Result<(), BenchError> {
            self.events.borrow_mut().push("prepare");
            if self.fail_prepare {
                Err(BenchError::Runtime("prepare fixture".into()))
            } else {
                Ok(())
            }
        }
        fn run(&mut self) -> Result<(), BenchError> {
            self.events.borrow_mut().push("run");
            if self.fail {
                Err(BenchError::Runtime("fixture".into()))
            } else {
                Ok(())
            }
        }
        fn consume(&self) {
            self.events.borrow_mut().push("observe");
        }
        fn finish_sample(&mut self) {
            self.events.borrow_mut().push("finish");
        }
    }

    #[test]
    fn interactive_failure_observes_before_cleanup() {
        let mut workload = Tracked {
            events: RefCell::default(),
            fail: true,
            fail_prepare: false,
        };
        assert!(interactive_sample(&mut workload).is_err());
        assert_eq!(
            *workload.events.borrow(),
            ["prepare", "run", "observe", "finish"]
        );
    }

    #[test]
    fn failed_preparation_still_finishes_without_running() {
        let mut workload = Tracked {
            events: RefCell::default(),
            fail: false,
            fail_prepare: true,
        };
        assert!(interactive_sample(&mut workload).is_err());
        assert_eq!(*workload.events.borrow(), ["prepare", "finish"]);
    }

    #[test]
    fn interactive_warmup_discard_and_samples_share_lifecycle() {
        struct Discard;
        impl MeasurementObserver for Discard {
            fn measurement_progress(
                &mut self,
                progress: MeasurementProgress,
                _: &[f64],
                _: (u32, u32),
            ) -> bool {
                progress.samples_done == 0
            }
        }
        let flags = Flags::parse(
            &[
                "--sample-mode",
                "interactive",
                "--sample-size",
                "2",
                "--warmup-iterations",
                "1",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        let config = MeasurementConfig::from_flags(&flags).unwrap();
        let mut workload = Tracked {
            events: RefCell::default(),
            fail: false,
            fail_prepare: false,
        };
        let result = measure_workload(&mut workload, (1, 1), &config, &mut Discard).unwrap();
        assert_eq!(result.sample_ns.len(), 2);
        assert_eq!(result.iterations_per_sample, 1);
        let events = workload.events.borrow();
        assert_eq!(events.len(), 17); // Warmup, discard, two samples, final observation.
        for sample in events[..16].chunks_exact(4) {
            assert_eq!(sample, ["prepare", "run", "observe", "finish"]);
        }
    }
}
