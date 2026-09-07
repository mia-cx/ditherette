//! Public browser measurements with a separately executed frozen Rust reference.

use crate::{error::BenchError, fixture::Fixture, measure::run_resize_once, registry::Registry};
use ditherette_bench::{
    browser_worker::{preserve_reference_mismatch, run_transport},
    lease::Lease,
    paired::{
        browser::*,
        coordinator::{live_benchmarks, validate_experiment},
        *,
    },
    verification::content_digest,
};
use ditherette_bench_api::{verification::*, ResizeAnchorParam, ResizeParams};
use std::{fs, io::Write, path::Path};

pub(crate) fn run(lease: &Lease, registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let [path] = args else {
        return Err(BenchError::Config(
            "paired-browser-trial requires one prepared request path".into(),
        ));
    };
    let mut request: TrialRequest =
        serde_json::from_slice(&fs::read(path).map_err(BenchError::io)?)
            .map_err(|error| BenchError::Config(error.to_string()))?;
    if request.reference_state != ReferenceState::Frozen || request.reference_output.is_some() {
        return Err(BenchError::Config(
            "browser worker requires frozen reference state and computes its own preflight output"
                .into(),
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
        return Err(BenchError::Config(
            "browser worker differs from its embedded clean revision or complete executable digest"
                .into(),
        ));
    }
    validate_experiment(&Experiment {
        label: "browser request".into(),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes: "prepared coordinator request".into(),
        cases: vec![request.case.clone()],
    })
    .map_err(BenchError::io)?;
    let case = &request.case;
    let browser = case
        .browser
        .as_ref()
        .ok_or_else(|| BenchError::Config("missing browser recipe".into()))?;
    let trial = request
        .browser
        .as_ref()
        .ok_or_else(|| BenchError::Config("missing browser assets".into()))?;
    validate_trial(trial).map_err(BenchError::io)?;
    let reference = registry.resize_subject(&case.reference_subject)?;
    let fixture = Fixture {
        id: case.name.clone(),
        kind: "paired-public-rgba8".into(),
        fingerprint: format!("{:02x?}", case.identity.input.0),
        width: case.source.width,
        height: case.source.height,
        rgba: case.rgba.clone(),
    };
    let params = ResizeParams {
        anchor: match browser.operation.anchor() {
            Anchor::TopLeft => ResizeAnchorParam::TopLeft,
            Anchor::Top => ResizeAnchorParam::Top,
            Anchor::TopRight => ResizeAnchorParam::TopRight,
            Anchor::Left => ResizeAnchorParam::Left,
            Anchor::Center => ResizeAnchorParam::Center,
            Anchor::Right => ResizeAnchorParam::Right,
            Anchor::BottomLeft => ResizeAnchorParam::BottomLeft,
            Anchor::Bottom => ResizeAnchorParam::Bottom,
            Anchor::BottomRight => ResizeAnchorParam::BottomRight,
        },
        ..ResizeParams::default()
    };
    let rgba = run_resize_once(
        &reference,
        &fixture,
        (case.identity.output.width, case.identity.output.height),
        &params,
    )?;
    let reference_output = VerificationOutput {
        dimensions: case.identity.output,
        pixels: Pixels::Rgba8 { data: rgba },
        warnings: vec![],
    };
    request.reference_output = Some(reference_output.clone());
    let transport_path = Path::new(path).with_extension("browser-request.json");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&transport_path)
        .map_err(BenchError::io)?
        .write_all(
            &serde_json::to_vec(&request).map_err(|error| BenchError::Config(error.to_string()))?,
        )
        .map_err(BenchError::io)?;
    let before = live_benchmarks().map_err(BenchError::io)?;
    let measured = run_transport(lease, &request, &transport_path).map_err(BenchError::io)?;
    let max_live = before.max(live_benchmarks().map_err(BenchError::io)?);
    let artifact = artifact_identity(&request.executable, &trial.assets, &trial.runtime)
        .map_err(BenchError::io)?;
    let record = |subject: &str, output| RecordedOutput {
        case: case.identity.clone(),
        implementation: ImplementationIdentity {
            subject: subject.into(),
            artifact: artifact.clone(),
        },
        output,
    };
    if measured.timing_skipped == Some(TimingSkipped::ReferenceMismatch) {
        preserve_reference_mismatch(
            &request,
            &measured,
            &Path::new(path).with_extension("preflight-mismatch"),
        )
        .map_err(BenchError::io)?;
        return Err(BenchError::Runtime(
            "untimed browser reference mismatch; retained S05 artifacts, no timing performed"
                .into(),
        ));
    }
    let result = TrialResult {
        role: request.role,
        pair: request.pair,
        case_name: case.name.clone(),
        build,
        measurement: case.measurement.clone(),
        warmup_iterations: measured.warmup_iterations,
        warmup_elapsed_ns: measured.warmup_elapsed_ns,
        sample_ns: measured.sample_ns,
        iterations_per_sample: measured.iterations_per_sample,
        reference: record(&case.reference_subject, reference_output),
        output: record(
            browser.operation.subject(browser.backend(request.role)),
            measured.output,
        ),
        pid: std::process::id(),
        max_live_benchmark_processes: max_live,
        browser: Some(BrowserEvidence {
            assets: trial.assets.tree.digest,
            runtime: runtime_digest(&trial.runtime).map_err(BenchError::io)?,
            backend: browser.backend(request.role),
            preparation: browser.preparation,
            cache: browser.cache,
            observation: measured.observation,
        }),
    };
    println!(
        "{}",
        serde_json::to_string(&result).map_err(|error| BenchError::Runtime(error.to_string()))?
    );
    Ok(())
}
