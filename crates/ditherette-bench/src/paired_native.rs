//! Native protocol adapter. Reuses the existing measurement loop and registry.

use crate::{
    case::ResizeScale,
    cli::Flags,
    error::BenchError,
    fixture::Fixture,
    measure::{
        measure_resize_case, measure_workload, run_resize_once, MeasurementConfig,
        MeasurementObserver, MeasurementProgress, Workload,
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
use ditherette_wasm::{
    bench_subjects::{
        diffusion, field_calls, fields, preparation, process, quantize as adapters,
        reference::ReferenceRequest, scalar, scores, yiluoma, BenchSubject,
    },
    image::{ImageDimensions, ImageView, Rgba8},
    prod::{color::packed::Converter, contract::request::QuantizeRequest},
    spec::contract::request as spec,
};
use std::{
    cell::RefCell,
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
    let build = BuildIdentity::current();
    if build.dirty
        || build.revision != request.executable.revision
        || content_digest(
            &fs::read(std::env::current_exe().map_err(BenchError::io)?).map_err(BenchError::io)?,
        ) != request.executable.content
    {
        return Err(BenchError::Config("paired artifact differs from its embedded clean source revision or complete executable digest".into()));
    }
    validate_native(&request.case, registry, request.role)?;
    if request.case.native.is_some() {
        return run_typed(registry, &request, build, std::path::Path::new(path));
    }
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
    if let Some(operation) = &case.native {
        let subject_id = match role {
            Role::Accepted => &case.accepted_subject,
            Role::Candidate => &case.candidate_subject,
        };
        let BenchSubject::Conformance(subject) = registry.subject(subject_id)? else {
            return Err(BenchError::Config(
                "typed native operation requires a conformance subject".into(),
            ));
        };
        let BenchSubject::Conformance(reference) = registry.subject(&case.reference_subject)?
        else {
            return Err(BenchError::Config(
                "typed native operation requires its conformance reference".into(),
            ));
        };
        let callable = match operation {
            native::NativeOperation::Processor { settings, .. } => settings.subject() == subject_id,
            native::NativeOperation::Process { .. } => process::callable(subject_id),
            native::NativeOperation::Diffusion { .. } => diffusion::function(subject_id).is_some(),
            native::NativeOperation::Yliluoma { .. } => {
                yiluoma::yiluoma_function(subject_id).is_some()
            }
            native::NativeOperation::FieldComponent { component } => {
                component.prod_subject() == subject_id
            }
            native::NativeOperation::Perturb { .. } => field_calls::PERTURB_SUBJECT == subject_id,
            native::NativeOperation::PerturbComponent { .. } => {
                scalar::PERTURB_SUBJECT == subject_id
            }
            native::NativeOperation::Separable { .. } => {
                field_calls::SEPARABLE_SUBJECT == subject_id
            }
            native::NativeOperation::MetricScores { metric } => metric.prod_subject() == subject_id,
            native::NativeOperation::Quantize { .. } => {
                adapters::quantize_function(subject_id).is_some()
            }
            native::NativeOperation::ColorForward { space } => {
                adapters::color_subject(*space).is_ok_and(|id| id == subject_id)
            }
        };
        let frozen = subject_id == &case.reference_subject
            && matches!(
                operation,
                native::NativeOperation::Quantize { .. }
                    | native::NativeOperation::Diffusion { .. }
                    | native::NativeOperation::Yliluoma { .. }
                    | native::NativeOperation::ColorForward { .. }
                    | native::NativeOperation::MetricScores { .. }
                    | native::NativeOperation::FieldComponent { .. }
                    | native::NativeOperation::PerturbComponent { .. }
            );
        let oracle = if frozen {
            Some(subject.descriptor.id.as_str())
        } else {
            subject
                .descriptor
                .default_oracle
                .as_ref()
                .map(|id| id.as_str())
        };
        if !(callable || frozen)
            || subject.operation != case.identity.semantics.operation
            || reference.operation != subject.operation
            || oracle != Some(case.reference_subject.as_str())
        {
            return Err(BenchError::Config(
                "native callable or oracle differs from its typed operation".into(),
            ));
        }
        return Ok(());
    }
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

enum TypedWorkload<'a> {
    FrozenIndexed(spec::Request<'a>),
    PerturbComponent {
        batch: scalar::PerturbBatch<'a>,
        production: bool,
    },
    Processor {
        call: preparation::CompleteCall<'a>,
        source: &'a [u8],
        processor: Option<ditherette_wasm::prod::pipeline::processor::Processor>,
        reset_each_sample: bool,
        sample_prime: Option<(preparation::CompleteCall<'a>, VerificationOutput)>,
        output: Option<preparation::Output>,
        audit: OutputAudit,
    },
    Process {
        call: process::CompleteCall<'a>,
        processor: ditherette_wasm::prod::pipeline::processor::Processor,
    },
    Diffusion {
        run: diffusion::DiffusionFn,
        request: ditherette_wasm::prod::contract::request::DitherQuantizeRequest<'a>,
    },
    Yliluoma {
        run: yiluoma::YliluomaFn,
        request: ditherette_wasm::prod::contract::request::DitherQuantizeRequest<'a>,
    },
    FieldComponent {
        batch: fields::PreparedComponent<'a>,
        production: bool,
    },
    CompleteField {
        call: field_calls::CompleteCall<'a>,
        processor: ditherette_wasm::prod::pipeline::processor::Processor,
    },
    Scores {
        run: scores::ScoreFn,
        pairs: Vec<scores::ScorePair>,
        values: Vec<f32>,
    },
    Quantize {
        run: adapters::QuantizeFn,
        request: QuantizeRequest<'a>,
    },
    Color {
        converter: Converter,
        frozen_space: Option<spec::WorkingSpace>,
        source: ImageView<'a, Rgba8>,
        coordinates: Vec<f32>,
    },
}

struct OutputAudit {
    expected: VerificationOutput,
    first_mismatch: RefCell<Option<VerificationOutput>>,
}

impl OutputAudit {
    fn observe(&self, output: VerificationOutput) {
        if output != self.expected && self.first_mismatch.borrow().is_none() {
            *self.first_mismatch.borrow_mut() = Some(output);
        }
    }
}

impl Workload for TypedWorkload<'_> {
    fn prepare_sample(&mut self) -> Result<(), BenchError> {
        if let Self::Processor {
            processor,
            reset_each_sample,
            output,
            sample_prime,
            source,
            ..
        } = self
        {
            *output = None;
            if *reset_each_sample {
                *processor =
                    Some(field_calls::processor().map_err(|e| BenchError::Runtime(e.to_string()))?);
                if let Some((prime, expected)) = sample_prime {
                    let actual = prime
                        .output(processor.as_mut().expect("new processor"), source)
                        .map_err(|e| BenchError::Runtime(e.to_string()))?
                        .verification();
                    if actual != *expected {
                        return Err(BenchError::Runtime(format!(
                            "stage prime differs from frozen reference: {}",
                            serde_json::to_string(&(expected, actual))
                                .map_err(|e| BenchError::Runtime(e.to_string()))?
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn finish_sample(&mut self) {
        if let Self::Processor {
            processor,
            reset_each_sample: true,
            ..
        } = self
        {
            // Dropping the owned instance releases it even when the call failed.
            *processor = None;
        }
    }

    fn run(&mut self) -> Result<(), BenchError> {
        match self {
            Self::FrozenIndexed(request) => {
                drop(std::hint::black_box(
                    scalar::indexed_call(*request)
                        .map_err(|e| BenchError::Runtime(e.to_string()))?,
                ));
            }
            Self::PerturbComponent { batch, production } => batch.run(*production),
            Self::Processor {
                call,
                source,
                processor,
                output,
                ..
            } => {
                *output = Some(
                    call.output(processor.as_mut().expect("prepared processor"), source)
                        .map_err(|e| BenchError::Runtime(e.to_string()))?,
                );
            }
            Self::Process { call, processor } => call
                .run(processor)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
            Self::Diffusion { run, request } | Self::Yliluoma { run, request } => {
                drop(std::hint::black_box(
                    run(*request).map_err(|e| BenchError::Runtime(e.to_string()))?,
                ));
            }
            Self::FieldComponent { batch, production } => batch.run_selected(*production),
            Self::CompleteField { call, processor } => call
                .run(processor)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
            Self::Scores { run, pairs, values } => {
                scores::score_into(std::hint::black_box(pairs.as_slice()), values, *run);
                // Every batch writes observable scores, including throughput iterations.
                std::hint::black_box(values.as_slice());
            }
            Self::Quantize { run, request } => {
                // Complete result construction and disposal remain inside every call.
                drop(std::hint::black_box(
                    run(*request).map_err(|e| BenchError::Runtime(e.to_string()))?,
                ));
            }
            Self::Color {
                converter,
                frozen_space,
                source,
                coordinates,
            } => {
                if let Some(space) = frozen_space {
                    scalar::forward_into(*source, coordinates, *space);
                } else {
                    converter.rgba8_into(*source, coordinates);
                }
                // Every conversion is observable, including earlier iterations in a throughput batch.
                std::hint::black_box(&*coordinates);
            }
        }
        Ok(())
    }
    fn consume(&self) {
        if let Self::Processor { output, audit, .. } = self {
            std::hint::black_box(output);
            if let Some(output) = output {
                audit.observe(output.verification());
            }
        }
        if let Self::Scores { values, .. } = self {
            std::hint::black_box(values);
        }
        if let Self::Color { coordinates, .. } = self {
            std::hint::black_box(coordinates);
        }
    }
}

fn run_typed(
    registry: &Registry,
    request: &TrialRequest,
    build: BuildIdentity,
    request_path: &std::path::Path,
) -> Result<(), BenchError> {
    let case = &request.case;
    let operation = case.native.as_ref().expect("validated typed operation");
    let subject_id = match request.role {
        Role::Accepted => &case.accepted_subject,
        Role::Candidate => &case.candidate_subject,
    };
    let parameters = operation
        .reference_request(case.source, &case.rgba)
        .map_err(BenchError::io)?;
    let verify = |id: &str| -> Result<VerificationOutput, BenchError> {
        let BenchSubject::Conformance(subject) = registry.subject(id)? else {
            unreachable!("validated conformance subject")
        };
        (subject.run)(&parameters).map_err(|e| BenchError::Runtime(e.to_string()))
    };
    let reference_output = verify(&case.reference_subject)?;
    let before = verify(subject_id)?;
    if matches!(operation, native::NativeOperation::Processor { .. }) && before != reference_output
    {
        let evidence = serde_json::to_vec_pretty(&(request, &reference_output, &before))
            .map_err(|e| BenchError::Runtime(e.to_string()))?;
        fs::write(
            request_path.with_extension("preflight-mismatch.json"),
            evidence,
        )
        .map_err(BenchError::io)?;
        return Err(BenchError::Runtime(
            "Processor differs from frozen reference before timing".into(),
        ));
    }
    if matches!(operation, native::NativeOperation::Process { .. }) {
        let counterpart = verify(if subject_id == process::PROCESS_SUBJECT {
            process::STAGED_SUBJECT
        } else {
            process::PROCESS_SUBJECT
        })?;
        if before != counterpart {
            let evidence =
                serde_json::to_vec_pretty(&(request, &reference_output, &before, counterpart))
                    .map_err(|e| BenchError::Runtime(e.to_string()))?;
            fs::write(
                request_path.with_extension("composition-mismatch.json"),
                evidence,
            )
            .map_err(BenchError::io)?;
            return Err(BenchError::Runtime(
                "Process differs from staged production before timing".into(),
            ));
        }
    }
    let frozen = subject_id == &case.reference_subject;
    let mut workload = match operation {
        native::NativeOperation::PerturbComponent { .. } => TypedWorkload::PerturbComponent {
            batch: scalar::PerturbBatch::new(&parameters)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
            production: !frozen,
        },
        native::NativeOperation::Quantize { .. }
        | native::NativeOperation::Diffusion { .. }
        | native::NativeOperation::Yliluoma { .. }
            if frozen =>
        {
            let ReferenceRequest::Processing(request) = parameters else {
                unreachable!("validated indexed request")
            };
            TypedWorkload::FrozenIndexed(request)
        }
        native::NativeOperation::Processor { settings, cache } => {
            let call = preparation::CompleteCall::new(&parameters)
                .map_err(|e| BenchError::Runtime(e.to_string()))?;
            let sample_prime = cache
                .sample_prime()
                .map(|prime| {
                    let parameters = settings
                        .prime_request(prime, case.source, &case.rgba)
                        .map_err(BenchError::io)?;
                    let subject = match parameters {
                        ReferenceRequest::Processing(spec::Request::Resize(_)) => {
                            "spec:resize:request:v1"
                        }
                        ReferenceRequest::Processing(spec::Request::Perturb(_)) => {
                            "spec:perturb:request:v1"
                        }
                        ReferenceRequest::Processing(spec::Request::DitherAndQuantize(_)) => {
                            "spec:dither-and-quantize:request:v1"
                        }
                        ReferenceRequest::Processing(spec::Request::Quantize(_)) => {
                            "spec:quantize:request:v1"
                        }
                        ReferenceRequest::Processing(spec::Request::Process(_)) => {
                            "spec:process:request:v1"
                        }
                        _ => unreachable!("prime processing request"),
                    };
                    let BenchSubject::Conformance(subject) = registry.subject(subject)? else {
                        unreachable!("frozen prime")
                    };
                    let expected = (subject.run)(&parameters)
                        .map_err(|e| BenchError::Runtime(e.to_string()))?;
                    let call = preparation::CompleteCall::new(&parameters)
                        .map_err(|e| BenchError::Runtime(e.to_string()))?;
                    Ok::<_, BenchError>((call, expected))
                })
                .transpose()?;
            let reset_each_sample = case.measurement.application_cache == ApplicationCache::Cold
                || sample_prime.is_some();
            let processor = if reset_each_sample {
                None
            } else {
                let mut processor =
                    field_calls::processor().map_err(|e| BenchError::Runtime(e.to_string()))?;
                let mut prime = case.rgba.clone();
                for pixel in prime.chunks_exact_mut(4) {
                    pixel[0] ^= 0xff;
                }
                drop(std::hint::black_box(
                    call.output(&mut processor, &prime)
                        .map_err(|e| BenchError::Runtime(e.to_string()))?,
                ));
                let primed_output = call
                    .output(&mut processor, &case.rgba)
                    .map_err(|e| BenchError::Runtime(e.to_string()))?
                    .verification();
                if primed_output != before {
                    let evidence = serde_json::to_vec_pretty(&(request, &before, &primed_output))
                        .map_err(|e| BenchError::Runtime(e.to_string()))?;
                    fs::write(
                        request_path.with_extension("primed-preflight-mismatch.json"),
                        evidence,
                    )
                    .map_err(BenchError::io)?;
                    return Err(BenchError::Runtime(
                        "primed Processor differs before timing".into(),
                    ));
                }
                Some(processor)
            };
            TypedWorkload::Processor {
                call,
                source: &case.rgba,
                processor,
                reset_each_sample,
                sample_prime,
                output: None,
                audit: OutputAudit {
                    expected: before.clone(),
                    first_mismatch: RefCell::new(None),
                },
            }
        }
        native::NativeOperation::Process { .. } => TypedWorkload::Process {
            call: process::CompleteCall::new(&parameters, subject_id)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
            processor: field_calls::processor().map_err(|e| BenchError::Runtime(e.to_string()))?,
        },
        native::NativeOperation::Diffusion { .. } => TypedWorkload::Diffusion {
            run: diffusion::function(subject_id).expect("validated diffusion callable"),
            request: diffusion::request(&parameters)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
        },
        native::NativeOperation::Yliluoma { .. } => TypedWorkload::Yliluoma {
            run: yiluoma::yiluoma_function(subject_id).expect("validated native callable"),
            request: yiluoma::yiluoma_request(&parameters)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
        },
        native::NativeOperation::FieldComponent { component } => TypedWorkload::FieldComponent {
            batch: fields::PreparedComponent::new(*component, parameters.source())
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
            production: !frozen,
        },
        native::NativeOperation::Perturb { .. } | native::NativeOperation::Separable { .. } => {
            TypedWorkload::CompleteField {
                call: field_calls::CompleteCall::new(&parameters)
                    .map_err(|e| BenchError::Runtime(e.to_string()))?,
                processor: field_calls::processor()
                    .map_err(|e| BenchError::Runtime(e.to_string()))?,
            }
        }
        native::NativeOperation::MetricScores { metric } => TypedWorkload::Scores {
            run: if frozen {
                metric.reference_function()
            } else {
                metric.prod_function()
            },
            pairs: scores::prepare_pairs(parameters.source(), *metric)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
            values: vec![0.0; case.rgba.len() / 4],
        },
        native::NativeOperation::Quantize { .. } => TypedWorkload::Quantize {
            run: adapters::quantize_function(subject_id).expect("validated native callable"),
            request: adapters::quantize_request(&parameters)
                .map_err(|e| BenchError::Runtime(e.to_string()))?,
        },
        native::NativeOperation::ColorForward { space } => TypedWorkload::Color {
            frozen_space: frozen.then_some(*space),
            converter: Converter::new(
                adapters::ordinary_space(*space).map_err(|e| BenchError::Runtime(e.to_string()))?,
            ),
            source: ImageView::packed(
                &case.rgba,
                ImageDimensions::new(case.source.width, case.source.height)
                    .map_err(|e| BenchError::Runtime(e.to_string()))?,
            )
            .map_err(|e| BenchError::Runtime(e.to_string()))?,
            coordinates: vec![0.0; case.rgba.len() / 4 * 3],
        },
    };
    let mut observer = Observer {
        warmup_iterations: 0,
        started: Instant::now(),
        warmup_elapsed_ns: 0,
        max_live: live_benchmarks().map_err(BenchError::io)?,
        observation_error: None,
    };
    let measured = measure_workload(
        &mut workload,
        (case.source.width, case.source.height),
        &config(&case.measurement)?,
        &mut observer,
    )?;
    if let Some(error) = observer.observation_error {
        return Err(BenchError::io(error));
    }
    let after = match &workload {
        TypedWorkload::Processor { output, audit, .. } => audit
            .first_mismatch
            .borrow()
            .clone()
            .unwrap_or_else(|| output.as_ref().expect("measured output").verification()),
        TypedWorkload::FieldComponent { batch, .. } => batch.output(),
        TypedWorkload::PerturbComponent { batch, .. } => batch.output(),
        TypedWorkload::Scores { values, .. } => VerificationOutput {
            dimensions: case.identity.output,
            pixels: Pixels::Scores {
                values: values.clone(),
            },
            warnings: vec![],
        },
        TypedWorkload::Color { coordinates, .. } => {
            let mut output = before.clone();
            let Pixels::Color {
                coordinates: actual,
                ..
            } = &mut output.pixels
            else {
                unreachable!("color verification output")
            };
            actual.clone_from(coordinates);
            output
        }
        _ => verify(subject_id)?,
    };
    if before != after {
        // Keep exact concrete evidence, but never publish unstable samples as a valid trial.
        let evidence = serde_json::to_vec_pretty(&(
            request,
            reference_output,
            before,
            after,
            &measured.sample_ns,
        ))
        .map_err(|e| BenchError::Runtime(e.to_string()))?;
        fs::write(request_path.with_extension("unstable.json"), evidence)
            .map_err(BenchError::io)?;
        return Err(BenchError::Runtime(
            "native output changed between untimed preflight and final verification".into(),
        ));
    }
    let record = |subject: String, output| RecordedOutput {
        case: case.identity.clone(),
        implementation: ImplementationIdentity {
            subject,
            artifact: request.executable.clone(),
        },
        output,
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
        reference: record(case.reference_subject.clone(), reference_output),
        output: record(subject_id.clone(), after),
        pid: std::process::id(),
        max_live_benchmark_processes: observer.max_live,
        browser: None,
    };
    println!(
        "{}",
        serde_json::to_string(&result).map_err(|e| BenchError::Runtime(e.to_string()))?
    );
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
        .map(|config| config.with_minimum_samples(5))
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
    fn scalar_plan_roles_validate_without_measurement() {
        use ditherette_bench::paired::scalar::{experiment, Comparison};
        let registry = Registry::load();
        for mode in [Comparison::SpecProd, Comparison::ProdProd] {
            for mut case in experiment(mode, "untimed fixture".into()).unwrap().cases {
                validate_native(&case, &registry, Role::Accepted).unwrap();
                validate_native(&case, &registry, Role::Candidate).unwrap();
                case.accepted_subject = "spec:process:request:v1".into();
                assert!(validate_native(&case, &registry, Role::Accepted).is_err());
            }
        }
    }

    #[test]
    fn stage_prime_mismatch_is_concrete_and_failed_setup_releases_processor() {
        let source = [1, 2, 3, 255];
        let settings = ditherette_bench::paired::preparation::ProcessorSettings::Resize {
            output: spec::Output {
                width: 1,
                height: 1,
                resize: spec::ResizePolicy::Nearest {
                    anchor: spec::Anchor::Center,
                },
            },
        };
        let request = settings
            .reference_request(
                Dimensions {
                    width: 1,
                    height: 1,
                },
                &source,
            )
            .unwrap();
        let expected = VerificationOutput {
            dimensions: Dimensions {
                width: 1,
                height: 1,
            },
            pixels: Pixels::Rgba8 {
                data: vec![99, 2, 3, 255],
            },
            warnings: vec![],
        };
        let mut workload = TypedWorkload::Processor {
            call: preparation::CompleteCall::new(&request).unwrap(),
            source: &source,
            processor: None,
            reset_each_sample: true,
            sample_prime: Some((
                preparation::CompleteCall::new(&request).unwrap(),
                expected.clone(),
            )),
            output: None,
            audit: OutputAudit {
                expected,
                first_mismatch: RefCell::new(None),
            },
        };
        let failure = workload.prepare_sample().unwrap_err().to_string();
        assert!(failure.contains("stage prime differs"));
        assert!(failure.contains("99,2,3,255"));
        assert!(failure.contains("1,2,3,255"));
        workload.finish_sample();
        assert!(matches!(
            workload,
            TypedWorkload::Processor {
                processor: None,
                output: None,
                ..
            }
        ));
    }

    #[test]
    fn processor_observation_retains_first_transient_mismatch() {
        let first = VerificationOutput {
            dimensions: Dimensions {
                width: 1,
                height: 1,
            },
            pixels: Pixels::Rgba8 {
                data: vec![1, 2, 3, 255],
            },
            warnings: vec![],
        };
        let mut distinct = first.clone();
        distinct.pixels = Pixels::Rgba8 {
            data: vec![2, 2, 3, 255],
        };
        let audit = OutputAudit {
            expected: first.clone(),
            first_mismatch: RefCell::new(None),
        };
        audit.observe(first.clone());
        audit.observe(distinct.clone());
        audit.observe(first);
        assert_eq!(*audit.first_mismatch.borrow(), Some(distinct));
    }

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

        let operation = native::NativeOperation::Quantize {
            settings: quantize::QuantizeSettings {
                palette: vec![quantize::PaletteEntry::Color { rgb: [1, 2, 3] }],
                alpha: quantize::AlphaPolicy::Premultiplied {},
                matching: quantize::MatchPolicy::SrgbEuclidean,
            },
        };
        case.identity = operation.identity(source, &case.rgba).unwrap();
        case.reference_subject = operation.reference_subject().into();
        case.measurement.scope = operation.scope();
        case.native = Some(operation);
        case.accepted_subject = adapters::QUANTIZE_SUBJECT.into();
        case.candidate_subject = "candidate:quantize:request:absent-in-this-artifact".into();
        validate_native(&case, &registry, Role::Accepted).unwrap();
        assert!(validate_native(&case, &registry, Role::Candidate).is_err());
        case.candidate_subject = adapters::QUANTIZE_SUBJECT.into();
        validate_native(&case, &registry, Role::Candidate).unwrap();
        case.candidate_subject = "spec:quantize:request:v1".into();
        validate_native(&case, &registry, Role::Candidate).unwrap();

        for metric in scores::MetricFamily::ALL {
            let operation = native::NativeOperation::MetricScores { metric };
            case.identity = operation.identity(source, &case.rgba).unwrap();
            case.reference_subject = operation.reference_subject().into();
            case.measurement.scope = operation.scope();
            case.native = Some(operation);
            case.accepted_subject = metric.prod_subject().into();
            case.candidate_subject = metric.prod_subject().into();
            validate_native(&case, &registry, Role::Candidate).unwrap();
            case.candidate_subject = adapters::QUANTIZE_SUBJECT.into();
            assert!(validate_native(&case, &registry, Role::Candidate).is_err());
            case.candidate_subject = metric.prod_subject().into();
            case.measurement.scope = CallScope::NativeForwardConversion;
            assert!(validate_native(&case, &registry, Role::Candidate).is_err());
        }
    }
}
