//! One owned Node transport per benchmark worker, with checked response identities.

use crate::{
    browser_assets::{validate_bundle_revision, validate_trial_assets},
    lease::Lease,
    paired::{browser::*, *},
};
use ditherette_bench_api::verification::{
    ImplementationIdentity, Pixels, RecordedOutput, ThreeWayOutputs,
};
use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

/// Run the transport once and retain its raw response before parsing or validation.
/// An early error drops the owned child, which terminates and reaps it.
pub fn run_transport(
    lease: &Lease,
    request: &TrialRequest,
    request_path: &Path,
) -> io::Result<BrowserTransportResult> {
    let trial = request
        .browser
        .as_ref()
        .ok_or_else(|| invalid("browser request lacks prepared assets"))?;
    validate_trial_assets(trial)?;
    validate_bundle_revision(&trial.assets, &request.executable.revision)?;
    let request_path = fs::canonicalize(request_path)?;
    let raw_path = request_path.with_extension("transport.json");
    let mut raw = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(raw_path)?;
    let mut command = Command::new(&trial.runtime.node.path);
    command
        .arg(trial.assets.tree.root.join(&trial.assets.entries.transport))
        .arg(&request_path)
        .env("DITHERETTE_BENCH_TRANSPORT", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let result = read_owned_transport(lease, command, response_limit(request)?, &mut raw)?;
    validate_trial_assets(trial)?;
    validate_response(request, &result)?;
    reject_unstable_output(
        request,
        &result,
        &request_path.with_extension("unstable-output"),
    )?;
    Ok(result)
}

/// The transport emits one bounded JSON line. Diagnostics belong on stderr.
pub fn read_owned_transport(
    lease: &Lease,
    command: Command,
    limit: u64,
    raw: &mut impl Write,
) -> io::Result<BrowserTransportResult> {
    let mut child = lease.spawn(command)?;
    let stdout = child
        .take_stdout()
        .ok_or_else(|| invalid("transport stdout was not piped"))?;
    let mut reader = BufReader::new(stdout).take(limit + 1);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    raw.write_all(line.as_bytes())?;
    raw.flush()?;
    if line.len() as u64 > limit || !line.ends_with('\n') {
        return Err(invalid(
            "transport response exceeds its bound or lacks one complete JSON line",
        ));
    }
    let result = serde_json::from_str(&line).map_err(io::Error::other)?;
    let mut trailing = String::new();
    reader.read_to_string(&mut trailing)?;
    raw.write_all(trailing.as_bytes())?;
    if !trailing.trim().is_empty() || reader.limit() == 0 {
        return Err(invalid("transport emitted extra output"));
    }
    if !child.wait()?.success() {
        return Err(invalid("browser transport exited unsuccessfully"));
    }
    Ok(result)
}

fn response_limit(request: &TrialRequest) -> io::Result<u64> {
    // JSON RGBA bytes need at most four characters each, plus structured evidence.
    // An exact preflight does not preclude later instability. Retain two actual results in every trial.
    let outputs = 2;
    u64::from(request.case.identity.output.width)
        .checked_mul(u64::from(request.case.identity.output.height))
        .and_then(|pixels| pixels.checked_mul(16 * outputs))
        .and_then(|bytes| bytes.checked_add(1024 * 1024))
        .and_then(|bytes| {
            (request.case.measurement.samples as u64)
                .checked_mul(32)
                .and_then(|samples| bytes.checked_add(samples))
        })
        .ok_or_else(|| invalid("transport response size overflow"))
}

/// Preserve the actual untimed mismatch, with no fabricated output for the absent peer role.
pub fn preserve_reference_mismatch(
    request: &TrialRequest,
    result: &BrowserTransportResult,
    directory: &Path,
) -> io::Result<()> {
    validate_response(request, result)?;
    if result.timing_skipped != Some(TimingSkipped::ReferenceMismatch) {
        return Err(invalid("expected untimed reference mismatch"));
    }
    preserve_output(request, result.output.clone(), directory)
}

/// Preserve both actual images, then fail before a normal paired result can be published.
/// Raw transport JSON remains available even if writing a review bundle fails.
pub fn reject_unstable_output(
    request: &TrialRequest,
    result: &BrowserTransportResult,
    directory: &Path,
) -> io::Result<()> {
    validate_response(request, result)?;
    let Some(first) = &result.unstable_output else {
        return Ok(());
    };
    fs::create_dir(directory)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("transport.json"))?
        .write_all(&serde_json::to_vec_pretty(result).map_err(io::Error::other)?)?;
    preserve_output(request, first.clone(), &directory.join("first-output"))?;
    preserve_output(
        request,
        result.output.clone(),
        &directory.join("first-distinct-output"),
    )?;
    Err(invalid(
        "output changed during the trial; both actual outputs retained, trial rejected",
    ))
}

fn preserve_output(
    request: &TrialRequest,
    output: ditherette_bench_api::verification::VerificationOutput,
    directory: &Path,
) -> io::Result<()> {
    let trial = request
        .browser
        .as_ref()
        .ok_or_else(|| invalid("missing browser assets"))?;
    let browser = request
        .case
        .browser
        .as_ref()
        .ok_or_else(|| invalid("missing browser recipe"))?;
    let artifact = artifact_identity(&request.executable, &trial.assets, &trial.runtime)?;
    let record = |subject: &str, output| RecordedOutput {
        case: request.case.identity.clone(),
        implementation: ImplementationIdentity {
            subject: subject.into(),
            artifact: artifact.clone(),
        },
        output,
    };
    let actual = record(
        browser.operation.subject(browser.backend(request.role)),
        output,
    );
    let outputs = ThreeWayOutputs {
        reference_state: request.reference_state,
        reference: Some(record(
            &request.case.reference_subject,
            request
                .reference_output
                .clone()
                .ok_or_else(|| invalid("missing frozen output"))?,
        )),
        accepted: (request.role == Role::Accepted).then(|| actual.clone()),
        candidate: (request.role == Role::Candidate).then_some(actual),
    };
    crate::verification::verify_and_preserve(
        &request.case.identity,
        &outputs,
        crate::verification::VerificationBounds::exact(),
        directory,
    )?;
    Ok(())
}

/// Byte differences remain conformance failures for S05 review bundles, not protocol errors.
pub fn validate_response(
    request: &TrialRequest,
    result: &BrowserTransportResult,
) -> io::Result<()> {
    let trial = request
        .browser
        .as_ref()
        .ok_or_else(|| invalid("missing browser trial"))?;
    let case = &request.case;
    if request.reference_output.is_none() {
        return Err(invalid(
            "browser transport requires untimed frozen reference output",
        ));
    }
    validate_case(case)?;
    validate_observation(&trial.runtime, &result.observation)?;
    if result.role != request.role
        || result.pair != request.pair
        || result.case_name != case.name
        || result.input != case.identity.input
        || result.settings != case.identity.settings
    {
        return Err(invalid("browser response identity differs"));
    }
    if let Some(first) = &result.unstable_output {
        if result.timing_skipped.is_some() || *first == result.output {
            return Err(invalid("invalid first-distinct-output instability marker"));
        }
    }
    let indexed = matches!(
        &case
            .browser
            .as_ref()
            .expect("validated browser recipe")
            .operation,
        PublicOperation::Quantize { .. }
    );
    for output in std::iter::once(&result.output).chain(result.unstable_output.iter()) {
        let format_matches = if indexed {
            matches!(output.pixels, Pixels::Indexed8 { .. })
        } else {
            matches!(output.pixels, Pixels::Rgba8 { .. }) && output.warnings.is_empty()
        };
        if output.dimensions != case.identity.output || !format_matches {
            return Err(invalid(
                "browser result has invalid shape, format, or warning metadata",
            ));
        }
        crate::verification::render_rgba(output).map_err(io::Error::other)?;
    }
    if result.timing_skipped == Some(TimingSkipped::ReferenceMismatch) {
        if !result.sample_ns.is_empty()
            || result.iterations_per_sample != 0
            || result.warmup_iterations != 0
            || result.warmup_elapsed_ns != 0
            || request.reference_output.as_ref() == Some(&result.output)
        {
            return Err(invalid(
                "reference mismatch claims measured work or equal output",
            ));
        }
        return Ok(());
    } else if result.sample_ns.len() < 5
        || result.sample_ns.len() > case.measurement.samples
        || result
            .sample_ns
            .iter()
            .any(|sample| !sample.is_finite() || *sample < 0.0)
        || result.iterations_per_sample == 0
        || result.warmup_iterations == 0
        || result.warmup_elapsed_ns == 0
        || (case.measurement.mode == SampleMode::SingleCall && result.iterations_per_sample != 1)
    {
        return Err(invalid(
            "browser response identity or sample evidence differs",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
