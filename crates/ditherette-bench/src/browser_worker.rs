//! One owned Node transport per benchmark worker, with checked response identities.

mod indexed_wire;

use crate::{
    browser_assets::{validate_bundle_revision, validate_oracle, validate_trial_assets},
    lease::Lease,
    paired::{browser::*, *},
};
use ditherette_bench_api::verification::{
    ArtifactIdentity, CaseIdentity, Digest256, Dimensions, ImplementationIdentity, Pixels,
    RecordedOutput, ThreeWayOutputs, VerificationOutput, Warning,
};
use serde::Serialize;
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
    validate_oracle(&trial.assets)?;
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
    let result =
        read_owned_transport_with(lease, command, response_limit(request)?, &mut raw, |line| {
            match request
                .case
                .browser
                .as_ref()
                .and_then(|case| case.retained_output_limit_bytes)
            {
                Some(limit) => indexed_wire::decode(line, request.case.identity.output, limit),
                None => serde_json::from_str(line).map_err(io::Error::other),
            }
        })?;
    validate_trial_assets(trial)?;
    validate_response(request, &result)?;
    write_reference_diagnostics(
        request,
        &result,
        &request_path.with_extension("reference-diagnostics.json"),
        artifact_identity(&request.executable, &trial.assets, &trial.runtime)?,
    )?;
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
    read_owned_transport_with(lease, command, limit, raw, |line| {
        serde_json::from_str(line).map_err(io::Error::other)
    })
}

fn read_owned_transport_with(
    lease: &Lease,
    command: Command,
    limit: u64,
    raw: &mut impl Write,
    decode: impl FnOnce(&str) -> io::Result<BrowserTransportResult>,
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
    let result = decode(&line)?;
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

/// Bound the complete Node response, including its independently computed target reference.
pub fn response_limit(request: &TrialRequest) -> io::Result<u64> {
    // JSON RGBA bytes need at most four characters each, plus structured evidence.
    // An exact preflight does not preclude later instability. Retain two actual results in every trial.
    let outputs = 3;
    let bytes_per_pixel = if request
        .case
        .browser
        .as_ref()
        .is_some_and(|case| case.retained_output_limit_bytes.is_some())
    {
        2 // Canonical hex carries one indexed byte, not four RGBA decimal bytes.
    } else {
        16
    };
    u64::from(request.case.identity.output.width)
        .checked_mul(u64::from(request.case.identity.output.height))
        .and_then(|pixels| pixels.checked_mul(bytes_per_pixel * outputs))
        .and_then(|bytes| bytes.checked_add(1024 * 1024))
        .and_then(|bytes| {
            (request.case.measurement.samples as u64)
                .checked_mul(32)
                .and_then(|samples| bytes.checked_add(samples))
        })
        .ok_or_else(|| invalid("transport response size overflow"))
}

fn uses_capped_diagnostics(request: &TrialRequest) -> bool {
    request
        .case
        .browser
        .as_ref()
        .is_some_and(|case| case.retained_output_limit_bytes.is_some())
}

#[derive(Serialize)]
struct CappedPixels<'a> {
    format: &'static str,
    index_count: usize,
    index_digest: Digest256,
    palette_rgba: &'a [u8],
    transparent_index: Option<u8>,
}

#[derive(Serialize)]
struct CappedOutput<'a> {
    dimensions: &'a Dimensions,
    pixels: CappedPixels<'a>,
    warnings: &'a [Warning],
}

fn capped_output(output: &VerificationOutput) -> io::Result<CappedOutput<'_>> {
    let Pixels::Indexed8 {
        indices,
        palette_rgba,
        transparent_index,
    } = &output.pixels
    else {
        return Err(invalid("capped diagnostics require indexed output"));
    };
    Ok(CappedOutput {
        dimensions: &output.dimensions,
        pixels: CappedPixels {
            format: "indexed8",
            index_count: indices.len(),
            index_digest: crate::verification::content_digest(indices),
            palette_rgba,
            transparent_index: *transparent_index,
        },
        warnings: &output.warnings,
    })
}

fn write_pretty(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer_pretty(file, value).map_err(io::Error::other)
}

fn write_reference_diagnostics(
    request: &TrialRequest,
    result: &BrowserTransportResult,
    path: &Path,
    artifact: ArtifactIdentity,
) -> io::Result<()> {
    if !uses_capped_diagnostics(request) {
        return OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(
                &serde_json::to_vec_pretty(&serde_json::json!({
                "case": request.case.identity,
                "native_reference": request.reference_output,
                "wasm_reference": result.reference,
                "artifact": artifact,
                }))
                .map_err(io::Error::other)?,
            );
    }
    #[derive(Serialize)]
    struct Diagnostics<'a> {
        schema: &'static str,
        case: &'a CaseIdentity,
        native_reference: CappedOutput<'a>,
        wasm_reference: CappedOracleOutput<'a>,
        artifact: ArtifactIdentity,
    }
    #[derive(Serialize)]
    struct CappedOracleOutput<'a> {
        case: &'a CaseIdentity,
        output: CappedOutput<'a>,
    }
    let native = request
        .reference_output
        .as_ref()
        .ok_or_else(|| invalid("missing native reference for capped diagnostics"))?;
    let wasm = result
        .reference
        .as_ref()
        .ok_or_else(|| invalid("missing Wasm reference for capped diagnostics"))?;
    write_pretty(
        path,
        &Diagnostics {
            schema: "ditherette-capped-reference-diagnostic-v1",
            case: &request.case.identity,
            native_reference: capped_output(native)?,
            wasm_reference: CappedOracleOutput {
                case: &wasm.case,
                output: capped_output(&wasm.output)?,
            },
            artifact,
        },
    )
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
    preserve_output(
        request,
        result.reference.as_ref().expect("validated oracle"),
        result.output.clone(),
        directory,
    )
}

/// Preserve full ordinary outputs or capped summaries, then reject the unstable trial.
/// Raw transport JSON remains available even if writing the derived diagnostics fails.
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
    if uses_capped_diagnostics(request) {
        // run_transport already persisted the exact compact response before this bounded derivative.
        #[derive(Serialize)]
        struct Trial<'a> {
            role: Role,
            pair: usize,
            case_name: &'a str,
            input: &'a Digest256,
            settings: &'a Digest256,
            sample_ns: &'a [f64],
            iterations_per_sample: usize,
            warmup_iterations: usize,
            warmup_elapsed_ns: u128,
            observation: &'a BrowserObservation,
            timing_skipped: Option<TimingSkipped>,
        }
        #[derive(Serialize)]
        struct Oracle<'a> {
            case: &'a CaseIdentity,
            output: CappedOutput<'a>,
        }
        #[derive(Serialize)]
        struct Outputs<'a> {
            native_reference: CappedOutput<'a>,
            wasm_reference: Oracle<'a>,
            first_output: CappedOutput<'a>,
            first_distinct_output: CappedOutput<'a>,
        }
        #[derive(Serialize)]
        struct Diagnostics<'a> {
            schema: &'static str,
            trial: Trial<'a>,
            outputs: Outputs<'a>,
        }
        let wasm = result
            .reference
            .as_ref()
            .ok_or_else(|| invalid("missing Wasm reference for capped diagnostics"))?;
        let native = request
            .reference_output
            .as_ref()
            .ok_or_else(|| invalid("missing native reference for capped diagnostics"))?;
        write_pretty(
            &directory.join("transport.json"),
            &Diagnostics {
                schema: "ditherette-capped-transport-diagnostic-v1",
                trial: Trial {
                    role: result.role,
                    pair: result.pair,
                    case_name: &result.case_name,
                    input: &result.input,
                    settings: &result.settings,
                    sample_ns: &result.sample_ns,
                    iterations_per_sample: result.iterations_per_sample,
                    warmup_iterations: result.warmup_iterations,
                    warmup_elapsed_ns: result.warmup_elapsed_ns,
                    observation: &result.observation,
                    timing_skipped: result.timing_skipped,
                },
                outputs: Outputs {
                    native_reference: capped_output(native)?,
                    wasm_reference: Oracle {
                        case: &wasm.case,
                        output: capped_output(&wasm.output)?,
                    },
                    first_output: capped_output(first)?,
                    first_distinct_output: capped_output(&result.output)?,
                },
            },
        )?;
        return Err(invalid(
            "output changed during the trial; both actual outputs retained, trial rejected",
        ));
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("transport.json"))?
        .write_all(&serde_json::to_vec_pretty(result).map_err(io::Error::other)?)?;
    preserve_output(
        request,
        result.reference.as_ref().expect("validated oracle"),
        first.clone(),
        &directory.join("first-output"),
    )?;
    preserve_output(
        request,
        result.reference.as_ref().expect("validated oracle"),
        result.output.clone(),
        &directory.join("first-distinct-output"),
    )?;
    Err(invalid(
        "output changed during the trial; both actual outputs retained, trial rejected",
    ))
}

fn preserve_output(
    request: &TrialRequest,
    reference: &ditherette_bench_api::verification::OracleOutput,
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
            reference.output.clone(),
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
    let reference = result
        .reference
        .as_ref()
        .ok_or_else(|| invalid("missing Wasm oracle reference"))?;
    if reference.case != case.identity {
        return Err(invalid("Wasm oracle case identity differs"));
    }
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
    let browser_case = case
        .browser
        .as_ref()
        .ok_or_else(|| invalid("browser transport requires a browser recipe"))?;
    let indexed = matches!(
        &browser_case.operation,
        PublicOperation::Quantize { .. }
            | PublicOperation::Separable { .. }
            | PublicOperation::Diffusion { .. }
            | PublicOperation::Yliluoma { .. }
            | PublicOperation::Process { .. }
    );
    for output in std::iter::once(&result.output)
        .chain(result.unstable_output.iter())
        .chain(std::iter::once(&reference.output))
    {
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
            || reference.output == result.output
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
        || !has_complete_browser_timing_evidence(
            &case.measurement,
            &result.sample_ns,
            result.iterations_per_sample,
            result.warmup_elapsed_ns,
        )
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
