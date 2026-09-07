//! Native protocol adapter. Reuses the existing measurement loop and registry.

use crate::{
    case::ResizeScale,
    cli::Flags,
    error::BenchError,
    fixture::Fixture,
    measure::{
        measure_resize_case, run_resize_once, MeasurementConfig, MeasurementObserver,
        MeasurementProgress,
    },
    registry::Registry,
};
use ditherette_bench::{
    paired::{
        coordinator::{live_benchmarks, validate_experiment},
        *,
    },
    verification::{content_digest, verify_with_bounds, VerificationBounds},
};
use ditherette_bench_api::{verification::*, ResizeParams};
use std::{
    fs,
    time::{Duration, Instant},
};

pub(crate) fn run(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let [path] = args else {
        return Err(BenchError::Config(
            "paired-trial requires one prepared request path".into(),
        ));
    };
    let request: TrialRequest = serde_json::from_slice(&fs::read(path).map_err(BenchError::io)?)
        .map_err(|error| BenchError::Config(error.to_string()))?;
    if request.browser.is_some() || request.reference_output.is_some() {
        return Err(BenchError::Config(
            "native worker rejects browser assets".into(),
        ));
    }
    let build = BuildIdentity {
        revision: env!("DITHERETTE_BENCH_REVISION").into(),
        dirty: env!("DITHERETTE_BENCH_DIRTY") != "false",
        rustc: env!("DITHERETTE_BENCH_RUSTC").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
    };
    if build.dirty
        || build.revision != request.executable.revision
        || content_digest(
            &fs::read(std::env::current_exe().map_err(BenchError::io)?).map_err(BenchError::io)?,
        ) != request.executable.content
    {
        return Err(BenchError::Config("paired artifact differs from its embedded clean source revision or complete executable digest".into()));
    }
    validate_native(&request.case, registry, request.role)?;
    let case = &request.case;
    let subject_id = match request.role {
        Role::Accepted => &case.accepted_subject,
        Role::Candidate => &case.candidate_subject,
    };
    let reference = registry.resize_subject(&case.reference_subject)?;
    let subject = registry.resize_subject(subject_id)?;
    let fixture = Fixture {
        id: case.name.clone(),
        kind: "paired-rgba8".into(),
        fingerprint: format!("{:02x?}", case.identity.input.0),
        width: case.source.width,
        height: case.source.height,
        rgba: case.rgba.clone(),
    };
    let output = (case.identity.output.width, case.identity.output.height);
    let params = ResizeParams::default();
    let reference_rgba = run_resize_once(&reference, &fixture, output, &params)?;
    let subject_rgba = run_resize_once(&subject, &fixture, output, &params)?;
    let proof = verify_with_bounds(&reference_rgba, &subject_rgba, VerificationBounds::exact());
    let config = config(&case.measurement)?;
    let mut observer = Observer {
        warmup_iterations: 0,
        started: Instant::now(),
        warmup_elapsed_ns: 0,
        max_live: live_benchmarks().map_err(BenchError::io)?,
        observation_error: None,
    };
    let measured = measure_resize_case(
        &subject,
        &fixture,
        output,
        ResizeScale {
            x: f64::from(output.0) / f64::from(fixture.width),
            y: f64::from(output.1) / f64::from(fixture.height),
        },
        &params,
        &config,
        Some(proof),
        &mut observer,
    )?;
    if let Some(error) = observer.observation_error {
        return Err(BenchError::io(error));
    }
    let record = |subject: String, rgba: Vec<u8>| RecordedOutput {
        case: case.identity.clone(),
        implementation: ImplementationIdentity {
            subject,
            artifact: request.executable.clone(),
        },
        output: VerificationOutput {
            dimensions: case.identity.output,
            pixels: Pixels::Rgba8 { data: rgba },
            warnings: Vec::new(),
        },
    };
    let result = TrialResult {
        role: request.role,
        pair: request.pair,
        case_name: case.name.clone(),
        build,
        measurement: case.measurement.clone(),
        warmup_iterations: observer.warmup_iterations,
        warmup_elapsed_ns: observer.warmup_elapsed_ns,
        sample_ns: measured.sample_ns,
        iterations_per_sample: measured.iterations_per_sample,
        reference: record(case.reference_subject.clone(), reference_rgba),
        output: record(subject_id.clone(), subject_rgba),
        pid: std::process::id(),
        max_live_benchmark_processes: observer.max_live,
        browser: None,
    };
    println!(
        "{}",
        serde_json::to_string(&result).map_err(|error| BenchError::Runtime(error.to_string()))?
    );
    Ok(())
}

fn validate_native(case: &PairCase, registry: &Registry, role: Role) -> Result<(), BenchError> {
    if case.browser.is_some() {
        return Err(BenchError::Config(
            "native worker rejects browser requests".into(),
        ));
    }
    validate_experiment(&Experiment {
        label: "native request".into(),
        reference_state: ReferenceState::PreFreeze,
        pairs: 2,
        host_load_notes: "coordinator request".into(),
        cases: vec![case.clone()],
    })
    .map_err(BenchError::io)?;
    let m = &case.measurement;
    if m.scope != CallScope::NativeKernel || m.application_cache != ApplicationCache::NotApplicable
    {
        return Err(BenchError::Config("native resize has no application cache and cannot claim complete-call or initialization measurements".into()));
    }
    let subject = registry.resize_subject(match role {
        Role::Accepted => &case.accepted_subject,
        Role::Candidate => &case.candidate_subject,
    })?;
    registry.resize_subject(&case.reference_subject)?;
    let descriptor = &subject.descriptor;
    let oracle = if descriptor.id.module() == "spec" {
        Some(&descriptor.id)
    } else {
        descriptor.default_oracle.as_ref()
    };
    if oracle.map(|id| id.as_str()) != Some(case.reference_subject.as_str())
        || case.identity
            != native::identity(
                &case.reference_subject,
                case.source,
                &case.rgba,
                case.identity.output,
            )
            .map_err(BenchError::io)?
    {
        return Err(BenchError::Config(
            "native subject or normalized recipe identity differs".into(),
        ));
    }
    Ok(())
}

fn config(measurement: &Measurement) -> Result<MeasurementConfig, BenchError> {
    let args = vec![
        "--sample-size".into(),
        measurement.samples.to_string(),
        "--measurement-time-ms".into(),
        measurement.measurement_ms.to_string(),
        "--warm-up-time".into(),
        format!("{}ms", measurement.warmup_ms),
        "--target-sample-time".into(),
        measurement.target_sample_ms.to_string(),
        "--sample-mode".into(),
        match measurement.mode {
            SampleMode::SingleCall => "interactive",
            SampleMode::Throughput => "throughput",
        }
        .into(),
    ];
    MeasurementConfig::from_flags(&Flags::parse(&args)?)
}

struct Observer {
    warmup_iterations: usize,
    started: Instant,
    warmup_elapsed_ns: u128,
    max_live: usize,
    observation_error: Option<std::io::Error>,
}
impl MeasurementObserver for Observer {
    fn warmup_batch(&mut self, batch_size: usize, _: Duration) {
        self.warmup_iterations += batch_size;
    }
    fn measurement_progress(
        &mut self,
        progress: MeasurementProgress,
        _: &[f64],
        _: (u32, u32),
    ) -> bool {
        if progress.samples_done == 0 {
            self.warmup_elapsed_ns = self.started.elapsed().as_nanos();
        }
        // Process observation stays at measurement boundaries, outside samples.
        if progress.samples_done != 0
            && progress.samples_done < progress.sample_size
            && progress.elapsed < progress.measurement_time
        {
            return false;
        }
        match live_benchmarks() {
            Ok(count) => self.max_live = self.max_live.max(count),
            Err(error) => self.observation_error = Some(error),
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ditherette_bench::verification::settings_digest;

    #[test]
    fn native_adapter_rejects_claims_it_cannot_measure_without_timing() {
        let registry = Registry::load();
        let source = Dimensions {
            width: 1,
            height: 1,
        };
        let semantics = SemanticIdentity {
            operation: Operation::Resize,
            recipe: "nearest-center-default".into(),
            version: 1,
            space: None,
        };
        let mut case = PairCase {
            native: None,
            browser: None,
            name: "fixture".into(),
            source,
            rgba: vec![1, 2, 3, 255],
            identity: CaseIdentity {
                input: ditherette_bench::verification::input_digest(source, &[1, 2, 3, 255]),
                settings: settings_digest(&(semantics.clone(), source, "center-default")).unwrap(),
                semantics,
                output: source,
            },
            reference_subject: "spec:resize:nearest:scalar".into(),
            accepted_subject: "spec:resize:nearest:scalar".into(),
            candidate_subject: "prod:resize:nearest:scalar".into(),
            measurement: Measurement {
                mode: SampleMode::SingleCall,
                scope: CallScope::NativeKernel,
                application_cache: ApplicationCache::NotApplicable,
                samples: 5,
                measurement_ms: 20,
                warmup_ms: 1,
                target_sample_ms: 1,
            },
        };
        validate_native(&case, &registry, Role::Candidate).unwrap();
        assert_eq!(
            config(&case.measurement).unwrap().sample_mode().as_str(),
            "interactive"
        );
        case.measurement.mode = SampleMode::Throughput;
        assert_eq!(
            config(&case.measurement).unwrap().sample_mode().as_str(),
            "throughput"
        );
        case.measurement.application_cache = ApplicationCache::Cold;
        assert!(validate_native(&case, &registry, Role::Candidate).is_err());
        case.measurement.application_cache = ApplicationCache::NotApplicable;
        case.reference_subject = "spec:resize:bicubic:catmull-rom".into();
        case.accepted_subject = "prod:resize:bicubic:catmull-rom".into();
        case.candidate_subject = "prod:resize:bicubic:catmull-rom".into();
        case.identity =
            native::identity(&case.reference_subject, source, &case.rgba, source).unwrap();
        validate_native(&case, &registry, Role::Candidate).unwrap();
        case.candidate_subject = "prod:resize:bicubic:catmull-rom-scale-aware".into();
        assert!(validate_native(&case, &registry, Role::Candidate).is_err());
        case.candidate_subject = "prod:resize:bicubic:catmull-rom".into();
        case.identity.settings = content_digest(b"wrong settings");
        assert!(validate_native(&case, &registry, Role::Candidate).is_err());
    }
}
