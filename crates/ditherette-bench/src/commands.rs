//! CLI command implementations.

use std::{collections::BTreeMap, fs, path::Path};

use ditherette_bench_api::{ResizeBenchSubject, ResizeParams, SubjectId};

use crate::{
    baseline::{
        load_baseline, load_indexed_run, load_scoped_baseline,
        replace_scoped_baseline_from_indexed_run, save_baseline, save_indexed_run,
        save_scoped_baseline, write_json,
    },
    case::{scales_from_flags, ResizeScale},
    cli::{split_domain, Flags},
    compare::{
        acceptance_report, attach_accepted_comparisons, attach_measured_oracle_comparisons,
        attach_measured_spec_comparisons, attach_named_oracle_comparisons,
        attach_named_spec_comparisons, attach_pair_comparisons, attach_previous_comparisons,
        ensure_compatible_baseline, has_exact_comparisons, missing_exact_results,
    },
    error::BenchError,
    fixture::fixtures_from_flags,
    measure::{measure_resize_case, run_resize_once, MeasurementConfig},
    registry::Registry,
    report::{
        log_correctness_ok, log_correctness_start, log_perf_start, log_preheat_start,
        log_runtime_tuning, print_comp_table, print_correctness_failure_warning, print_perf_table,
        MeasurementLogger,
    },
    result::{verify_with_bounds, BenchResult, BenchRun, VerificationBounds, VerificationReport},
    runtime::{preheat_cpu, tune_runtime},
    util::{normalize_path_string, optional_id, output_dimensions},
};

#[derive(Debug, Clone)]
pub(crate) struct CorrectnessFailure {
    pub(crate) subject: SubjectId,
    pub(crate) oracle: SubjectId,
    pub(crate) case: String,
    pub(crate) verification: VerificationReport,
}

#[derive(Default)]
struct CorrectnessChecks {
    failures: Vec<CorrectnessFailure>,
    reports: BTreeMap<(SubjectId, String), VerificationReport>,
}

#[cfg(test)]
mod verification_tests {
    use super::*;
    use crate::baseline::save_baseline;

    #[test]
    fn bounded_failure_cannot_replace_accepted_baselines_when_failures_are_allowed() {
        let registry = Registry::load();
        let oracle = registry
            .resize_subject("spec:resize:nearest:scalar")
            .unwrap();
        let mut candidate = oracle.clone();
        candidate.descriptor.id = SubjectId::parse("candidate:resize:nearest:scalar").unwrap();
        candidate.resize_u8_rgba = |input, output, _| {
            output.data.copy_from_slice(input.data);
            output.data[0] += 1;
            Ok(())
        };
        let flags = Flags::parse(&[
            "--fixtures".into(),
            "solid".into(),
            "--size".into(),
            "1x1".into(),
            "--allow-correctness-failures".into(),
        ])
        .unwrap();
        let fixtures = fixtures_from_flags(&flags).unwrap();
        let scale = ResizeScale { x: 1.0, y: 1.0 };
        let checks = run_resize_correctness_checks(
            &registry,
            &oracle.descriptor.id,
            &[candidate.clone()],
            &fixtures,
            &[scale],
            VerificationBounds::bounded_default(),
            flags.optional("--allow-correctness-failures") == Some("true"),
        )
        .unwrap();
        assert_eq!(checks.failures.len(), 1);
        let mut result = resize_probe_result(&candidate, &fixtures[0], (1, 1), scale);
        result.verification = checks
            .reports
            .get(&(candidate.descriptor.id, result.case_id.clone()))
            .cloned();
        assert!(result.verification.as_ref().unwrap().within_bounds);
        assert!(!result.verification.as_ref().unwrap().passed);
        let mut run = BenchRun::new("perf", "resize", None, vec![result]);
        run.cli.push("--allow-correctness-failures".into());
        let name = format!("nonexact-verification-fixture-{}", std::process::id());
        assert!(save_scoped_baseline("accepted", &name, &run, true).is_err());

        let mut exact = run.clone();
        exact.results[0].verification = Some(verify_with_bounds(
            &fixtures[0].rgba,
            &fixtures[0].rgba,
            VerificationBounds::exact(),
        ));
        exact.results[0].verified = true;
        exact.results[0].output_digest = Some(ditherette_bench::verification::content_digest(
            &fixtures[0].rgba,
        ));
        save_baseline("accepted", &name, &exact, false).unwrap();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/bench/baselines/accepted")
            .join(format!("{name}.json"));
        let previous = fs::read(&path).unwrap();
        assert!(save_baseline("accepted", &name, &run, true).is_err());
        assert_eq!(fs::read(&path).unwrap(), previous);
        run.results[0].verification = None;
        assert!(save_baseline("accepted", &name, &run, true).is_err());
        fs::remove_file(path).unwrap();
    }
}

pub(crate) fn list_subjects(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let flags = Flags::parse(args)?;
    let domain = flags.optional("--domain");
    let module = flags.optional("--module");
    let path = flags.optional("--path").map(normalize_path_string);

    for subject in registry.subjects() {
        let descriptor = subject.descriptor();
        if domain.is_some_and(|domain| descriptor.id.domain() != domain) {
            continue;
        }
        if module.is_some_and(|module| descriptor.id.module() != module) {
            continue;
        }
        if let Some(path) = &path {
            if normalize_path_string(&descriptor.source_file) != *path {
                continue;
            }
        }
        println!(
            "{}\t{}\t{}",
            descriptor.id, descriptor.display_name, descriptor.source_file
        );
    }

    Ok(())
}

pub(crate) fn describe_subject(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let id = args
        .first()
        .ok_or_else(|| BenchError::Config("expected subject id".to_owned()))?;
    let subject = registry.resize_subject(id)?;
    let descriptor = &subject.descriptor;

    println!("id: {}", descriptor.id);
    println!("display: {}", descriptor.display_name);
    println!(
        "source: {}:{}",
        descriptor.source_file, descriptor.source_line
    );
    println!(
        "default_oracle: {}",
        optional_id(&descriptor.default_oracle)
    );
    println!(
        "pixel_formats: {}",
        descriptor
            .capabilities
            .pixel_formats
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "supports_tiling_params: {}",
        descriptor.capabilities.supports_tiling_params
    );
    println!(
        "scalar_control: {}",
        optional_id(&descriptor.capabilities.scalar_control)
    );

    Ok(())
}

pub(crate) fn perf_command(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let (domain, rest) = split_domain(args)?;
    if domain != "resize" {
        return Err(BenchError::Config(format!(
            "perf currently supports only resize, got {domain:?}"
        )));
    }

    let flags = Flags::parse(rest)?;
    if flags.optional("--save-baseline").is_some() && flags.optional("--replace-baseline").is_some()
    {
        return Err(BenchError::Config(
            "--save-baseline and --replace-baseline are mutually exclusive".to_owned(),
        ));
    }
    if flags.present("--no-run") && flags.optional("--replace-baseline").is_none() {
        return Err(BenchError::Config(
            "--no-run is only valid with --replace-baseline".to_owned(),
        ));
    }
    let save_baseline_name = flags.optional("--save-baseline");

    let mut subjects = registry.select_resize_subjects(&flags)?;
    if subjects.is_empty() {
        return Err(BenchError::Config("no resize subjects selected".to_owned()));
    }

    let save_oracle = flags
        .optional("--save-oracle")
        .or_else(|| flags.optional("--replace-oracle"));
    let oracle_arg = match save_oracle {
        Some("true") => flags.optional("--oracle").ok_or_else(|| {
            BenchError::Config(
                "--save-oracle without a subject requires --oracle SUBJECT".to_owned(),
            )
        })?,
        Some(subject) => subject,
        None => flags.optional("--oracle").unwrap_or(""),
    };
    let oracle_subject = (!oracle_arg.is_empty())
        .then(|| registry.resize_subject(oracle_arg))
        .transpose()?;
    let oracle_id = oracle_subject
        .as_ref()
        .map(|subject| subject.descriptor.id.clone());

    let spec_baseline = flags.optional("--spec-baseline");
    let mut measured_spec_subject: Option<SubjectId> = None;
    let loaded_spec_baseline = if let Some(value) = spec_baseline {
        if let Ok(id) = SubjectId::parse(value) {
            let spec_subject = registry.resize_subject(id.as_str())?;
            if !subjects
                .iter()
                .any(|subject| subject.descriptor.id == spec_subject.descriptor.id)
            {
                subjects.push(spec_subject);
            }
            measured_spec_subject = Some(id);
            None
        } else {
            Some(load_baseline("spec", value)?)
        }
    } else {
        None
    };

    let fixtures = fixtures_from_flags(&flags)?;
    let scales = scales_from_flags(&flags)?;
    let measurement = MeasurementConfig::from_flags(&flags)?;
    let verification_bounds = verification_bounds_from_flags(&flags)?;
    let allow_correctness_failures = flags
        .optional("--allow-correctness-failures")
        .is_some_and(|value| value == "true");
    if let Some(name) = flags.optional("--replace-baseline") {
        let probe_run = resize_probe_run(&subjects, &fixtures, &scales, &measurement, domain);
        let replaced = replace_scoped_baseline_from_indexed_run("accepted", name, &probe_run)?;
        println!(
            "replaced accepted baseline {name:?} from {replaced} latest compatible perf/{domain} case runs"
        );
        return Ok(());
    }

    let accepted_baseline_name = flags.optional("--baseline");
    let previous_run = if accepted_baseline_name.is_none() {
        let probe_run = resize_probe_run(&subjects, &fixtures, &scales, &measurement, domain);
        load_indexed_run(&probe_run)?
    } else {
        None
    };

    log_perf_start(
        domain,
        &subjects
            .iter()
            .map(|subject| subject.descriptor.id.to_string())
            .collect::<Vec<_>>(),
        &fixtures,
        &scales,
        &measurement,
        oracle_id.as_ref().map(SubjectId::as_str),
        accepted_baseline_name,
    );
    let runtime_report = tune_runtime(measurement.process_priority());
    log_runtime_tuning(&runtime_report);
    log_preheat_start(measurement.preheat_time());
    preheat_cpu(measurement.preheat_time());

    let oracle_baseline = oracle_subject
        .as_ref()
        .map(|oracle| {
            ensure_oracle_baseline(
                oracle,
                &subjects,
                &fixtures,
                &scales,
                &measurement,
                domain,
                save_oracle.is_some(),
            )
        })
        .transpose()?;

    if let Some(output_dir) = flags.optional("--output-img") {
        write_resize_output_images(
            Path::new(output_dir),
            oracle_subject.as_ref(),
            &subjects,
            &fixtures,
            &scales,
        )?;
    }

    let correctness = if let Some(oracle_id) = oracle_id.as_ref() {
        run_resize_correctness_checks(
            registry,
            oracle_id,
            &subjects,
            &fixtures,
            &scales,
            verification_bounds,
            allow_correctness_failures,
        )?
    } else {
        CorrectnessChecks::default()
    };

    let mut results = Vec::new();
    for fixture in &fixtures {
        for scale in &scales {
            let output = output_dimensions(fixture.width, fixture.height, scale.x, scale.y);
            for subject in &subjects {
                let case = format!("{}-{}x{}-{}", fixture.id, output.0, output.1, scale.label());
                let mut logger =
                    MeasurementLogger::new(&subject.descriptor.id.to_string(), &case, measurement);
                let mut result = measure_resize_case(
                    subject,
                    fixture,
                    output,
                    *scale,
                    &ResizeParams::default(),
                    &measurement,
                    correctness
                        .reports
                        .get(&(subject.descriptor.id.clone(), case.clone()))
                        .cloned(),
                    &mut logger,
                )?;
                if let Some(name) = accepted_baseline_name {
                    let probe_run = BenchRun::new(
                        "perf",
                        domain,
                        Some(measurement.artifact()),
                        vec![result.clone()],
                    );
                    let accepted_baseline = load_scoped_baseline("accepted", name, &probe_run)?;
                    ensure_compatible_baseline(&accepted_baseline, "perf", domain, &measurement)?;
                    attach_accepted_comparisons(
                        std::slice::from_mut(&mut result),
                        Some(&accepted_baseline),
                    );
                }
                logger.finish(&result);
                results.push(result);
            }
        }
    }

    let mut run = BenchRun::new("perf", domain, Some(measurement.artifact()), results);
    if let Some(name) = accepted_baseline_name {
        let accepted_baseline = load_scoped_baseline("accepted", name, &run)?;
        ensure_compatible_baseline(&accepted_baseline, "perf", domain, &measurement)?;
        attach_accepted_comparisons(&mut run.results, Some(&accepted_baseline));
    } else if let Some(previous_run) = previous_run.as_ref() {
        if has_exact_comparisons(&run.results, previous_run) {
            attach_previous_comparisons(&mut run.results, previous_run);
        }
    }
    if let Some(oracle_baseline) = oracle_baseline.as_ref() {
        attach_named_oracle_comparisons(&mut run.results, oracle_baseline);
    } else if let Some(oracle_id) = oracle_id.as_ref() {
        attach_measured_oracle_comparisons(&mut run.results, oracle_id);
    }
    if let Some(spec_subject) = measured_spec_subject.as_ref() {
        attach_measured_spec_comparisons(&mut run.results, spec_subject);
    }
    if let Some(spec_baseline) = loaded_spec_baseline.as_ref() {
        attach_named_spec_comparisons(&mut run.results, spec_baseline);
    }

    print_perf_table(&run.results);
    print_correctness_failure_warning(&correctness.failures);
    let acceptance = acceptance_report(&run.results, &flags)?;

    save_indexed_run(&run, save_baseline_name)?;
    if let Some(out) = flags.optional("--out") {
        write_json(out, &run)?;
    }
    if let Some(name) = save_baseline_name {
        save_scoped_baseline("accepted", name, &run, true)?;
    }
    if let Some(name) = flags.optional("--save-spec-baseline") {
        save_baseline("spec", name, &run, false)?;
    }
    if let Some(name) = flags.optional("--replace-spec-baseline") {
        save_baseline("spec", name, &run, true)?;
    }

    if acceptance.passed {
        Ok(())
    } else {
        Err(BenchError::Acceptance(acceptance.message))
    }
}

fn write_resize_output_images(
    output_dir: &Path,
    oracle: Option<&ResizeBenchSubject>,
    subjects: &[ResizeBenchSubject],
    fixtures: &[crate::fixture::Fixture],
    scales: &[ResizeScale],
) -> Result<(), BenchError> {
    let mut written = 0usize;

    for fixture in fixtures {
        for scale in scales {
            let output = output_dimensions(fixture.width, fixture.height, scale.x, scale.y);
            let case_dir = output_dir
                .join(sanitize_path_segment(&fixture.id))
                .join(format!("{}x{}-{}", output.0, output.1, scale.label()));

            if let Some(oracle) = oracle {
                let rgba = run_resize_once(oracle, fixture, output, &ResizeParams::default())?;
                let path = case_dir.join(format!(
                    "oracle__{}.png",
                    sanitize_path_segment(oracle.descriptor.id.as_str())
                ));
                write_rgba_png(&path, output, &rgba)?;
                written += 1;
            }

            for subject in subjects {
                if oracle.is_some_and(|oracle| oracle.descriptor.id == subject.descriptor.id) {
                    continue;
                }

                let rgba = run_resize_once(subject, fixture, output, &ResizeParams::default())?;
                let path = case_dir.join(format!(
                    "perf__{}.png",
                    sanitize_path_segment(subject.descriptor.id.as_str())
                ));
                write_rgba_png(&path, output, &rgba)?;
                written += 1;
            }
        }
    }

    println!(
        "{} wrote {written} PNG resize output{} under {}",
        crate::util::heading("Images"),
        if written == 1 { "" } else { "s" },
        output_dir.display()
    );
    Ok(())
}

fn write_rgba_png(path: &Path, output: (u32, u32), rgba: &[u8]) -> Result<(), BenchError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(BenchError::io)?;
    }
    image::save_buffer_with_format(
        path,
        rgba,
        output.0,
        output.1,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|error| {
        BenchError::Runtime(format!(
            "failed to write resize output {}: {error}",
            path.display()
        ))
    })
}

fn sanitize_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => character,
            _ => '-',
        })
        .collect()
}

const ORACLE_BASELINE_NAME: &str = "oracle";

fn ensure_oracle_baseline(
    oracle: &ResizeBenchSubject,
    subjects: &[ResizeBenchSubject],
    fixtures: &[crate::fixture::Fixture],
    scales: &[ResizeScale],
    measurement: &MeasurementConfig,
    domain: &str,
    force_refresh: bool,
) -> Result<BenchRun, BenchError> {
    if subjects
        .iter()
        .any(|subject| subject.descriptor.id.filter() != oracle.descriptor.id.filter())
    {
        return Err(BenchError::Config(format!(
            "oracle {} cannot compare subjects with a different filter",
            oracle.descriptor.id
        )));
    }

    let expected = oracle_probe_run(oracle, fixtures, scales, measurement, domain);
    let cached = if force_refresh {
        let mut empty = expected.as_baseline("oracle", ORACLE_BASELINE_NAME);
        empty.results.clear();
        empty
    } else {
        load_scoped_baseline("oracle", ORACLE_BASELINE_NAME, &expected)?
    };
    let missing = missing_exact_results(&expected.results, &cached);
    if missing.is_empty() {
        return Ok(cached);
    }

    let reason = if force_refresh {
        "refresh requested"
    } else {
        "missing cases"
    };
    println!(
        "{} oracle baseline {reason}; measuring {}",
        crate::util::heading("Oracle"),
        oracle.descriptor.id
    );

    let mut measured = Vec::new();
    for missing_case in &missing {
        let fixture = fixtures
            .iter()
            .find(|fixture| {
                fixture.id == missing_case.fixture
                    && fixture.fingerprint == missing_case.fixture_fingerprint
            })
            .expect("missing oracle case should refer to a requested fixture");
        let output = (missing_case.output_width, missing_case.output_height);
        let scale = result_scale(missing_case);
        let case = format!(
            "oracle-baseline/{}-{}x{}-{}",
            fixture.id,
            output.0,
            output.1,
            scale.label()
        );
        let mut logger =
            MeasurementLogger::new(&oracle.descriptor.id.to_string(), &case, *measurement);
        let result = measure_resize_case(
            oracle,
            fixture,
            output,
            scale,
            &ResizeParams::default(),
            measurement,
            None,
            &mut logger,
        )?;
        logger.finish(&result);
        measured.push(result);
    }

    let mut measured_run = BenchRun::new("perf", domain, Some(measurement.artifact()), measured);
    save_scoped_baseline("oracle", ORACLE_BASELINE_NAME, &measured_run, true)?;
    measured_run.results.extend(cached.results);
    Ok(measured_run.as_baseline("oracle", ORACLE_BASELINE_NAME))
}

fn resize_probe_run(
    subjects: &[ResizeBenchSubject],
    fixtures: &[crate::fixture::Fixture],
    scales: &[ResizeScale],
    measurement: &MeasurementConfig,
    domain: &str,
) -> BenchRun {
    let mut results = Vec::new();
    for fixture in fixtures {
        for scale in scales {
            let output = output_dimensions(fixture.width, fixture.height, scale.x, scale.y);
            for subject in subjects {
                results.push(resize_probe_result(subject, fixture, output, *scale));
            }
        }
    }
    BenchRun::new("perf", domain, Some(measurement.artifact()), results)
}

fn oracle_probe_run(
    oracle: &ResizeBenchSubject,
    fixtures: &[crate::fixture::Fixture],
    scales: &[ResizeScale],
    measurement: &MeasurementConfig,
    domain: &str,
) -> BenchRun {
    resize_probe_run(
        std::slice::from_ref(oracle),
        fixtures,
        scales,
        measurement,
        domain,
    )
}

fn resize_probe_result(
    subject: &ResizeBenchSubject,
    fixture: &crate::fixture::Fixture,
    output: (u32, u32),
    scale: ResizeScale,
) -> BenchResult {
    BenchResult {
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
        verified: false,
        verification: None,
        output_digest: None,
        checksum: String::new(),
        samples: 0,
        sample_ns: Vec::new(),
        iterations_per_sample: 0,
        total_iterations: 0,
        min_ns: 0.0,
        median_ns: 0.0,
        mean_ns: 0.0,
        stdev_ns: 0.0,
        mode_ns: 0.0,
        p75_ns: 0.0,
        p90_ns: 0.0,
        p95_ns: 0.0,
        p99_ns: 0.0,
        max_ns: 0.0,
        output_mpix_per_s: 0.0,
        comparisons: BTreeMap::new(),
    }
}

fn result_scale(result: &BenchResult) -> ResizeScale {
    ResizeScale {
        x: if result.scale_x == 0.0 {
            result.scale
        } else {
            result.scale_x
        },
        y: if result.scale_y == 0.0 {
            result.scale
        } else {
            result.scale_y
        },
    }
}

fn verification_bounds_from_flags(flags: &Flags) -> Result<VerificationBounds, BenchError> {
    let has_explicit_bound = flags.present("--max-color-distance")
        || flags.present("--max-mean-color-distance")
        || flags.present("--max-rms-color-distance");
    let mode = flags
        .optional("--correctness")
        .unwrap_or(if has_explicit_bound {
            "bounded"
        } else {
            "exact"
        });
    let mut bounds = match mode {
        "exact" => VerificationBounds::exact(),
        "bounded" | "approx" | "tolerant" => VerificationBounds::bounded_default(),
        other => {
            return Err(BenchError::Config(format!(
                "--correctness must be exact or bounded, got {other:?}"
            )))
        }
    };

    if let Some(value) = flags.optional("--max-color-distance") {
        bounds.max_color_distance = parse_nonnegative_f64(value, "--max-color-distance")?;
    }
    if let Some(value) = flags.optional("--max-mean-color-distance") {
        bounds.max_mean_color_distance = parse_nonnegative_f64(value, "--max-mean-color-distance")?;
    }
    if let Some(value) = flags.optional("--max-rms-color-distance") {
        bounds.max_rms_color_distance = parse_nonnegative_f64(value, "--max-rms-color-distance")?;
    }

    Ok(bounds)
}

fn parse_nonnegative_f64(value: &str, flag: &str) -> Result<f64, BenchError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| BenchError::Config(format!("invalid {flag} {value:?}")))?;
    if parsed.is_sign_negative() || !parsed.is_finite() {
        return Err(BenchError::Config(format!(
            "{flag} must be a non-negative finite number"
        )));
    }
    Ok(parsed)
}

fn run_resize_correctness_checks(
    registry: &Registry,
    oracle_id: &SubjectId,
    subjects: &[ResizeBenchSubject],
    fixtures: &[crate::fixture::Fixture],
    scales: &[ResizeScale],
    verification_bounds: VerificationBounds,
    allow_failures: bool,
) -> Result<CorrectnessChecks, BenchError> {
    let oracle = registry.resize_subject(oracle_id.as_str())?;
    let check_count = subjects
        .iter()
        .filter(|subject| subject.descriptor.id != *oracle_id)
        .count()
        * fixtures.len()
        * scales.len();
    log_correctness_start(check_count);
    let mut checks = CorrectnessChecks::default();

    for fixture in fixtures {
        for scale in scales {
            let output = output_dimensions(fixture.width, fixture.height, scale.x, scale.y);
            let oracle_output =
                run_resize_once(&oracle, fixture, output, &ResizeParams::default())?;
            let case = format!("{}-{}x{}-{}", fixture.id, output.0, output.1, scale.label());
            checks.reports.insert(
                (oracle_id.clone(), case.clone()),
                verify_with_bounds(&oracle_output, &oracle_output, VerificationBounds::exact()),
            );

            for subject in subjects {
                if subject.descriptor.id == *oracle_id {
                    continue;
                }
                let candidate_output =
                    run_resize_once(subject, fixture, output, &ResizeParams::default())?;
                let verification =
                    verify_with_bounds(&oracle_output, &candidate_output, verification_bounds);
                checks.reports.insert(
                    (subject.descriptor.id.clone(), case.clone()),
                    verification.clone(),
                );
                if !verification.passed {
                    if !allow_failures {
                        return Err(BenchError::Verify(format!(
                            "{} failed {} verification against {} for {}: {:?}; differing_pixels={}, max_color_distance={:.6}, mean_color_distance={:.6}, rms_color_distance={:.6}",
                            subject.descriptor.id,
                            verification.mode,
                            oracle_id,
                            case,
                            verification.first_mismatch,
                            verification.differing_pixels,
                            verification.max_color_distance,
                            verification.mean_color_distance,
                            verification.rms_color_distance
                        )));
                    }

                    checks.failures.push(CorrectnessFailure {
                        subject: subject.descriptor.id.clone(),
                        oracle: oracle_id.clone(),
                        case: case.clone(),
                        verification,
                    });
                    continue;
                }
                log_correctness_ok(
                    &subject.descriptor.id.to_string(),
                    oracle_id,
                    &case,
                    &verification,
                );
            }
        }
    }
    println!();
    Ok(checks)
}

pub(crate) fn comp_command(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let (domain, rest) = split_domain(args)?;
    if domain != "resize" {
        return Err(BenchError::Config(format!(
            "comp currently supports only resize, got {domain:?}"
        )));
    }

    let flags = Flags::parse(rest)?;
    let left = flags
        .optional("--left")
        .ok_or_else(|| BenchError::Config("expected --left".to_owned()))?;
    let right = flags
        .optional("--right")
        .ok_or_else(|| BenchError::Config("expected --right".to_owned()))?;
    let left = registry.resolve_resize_subject_or_path(left)?;
    let right = registry.resolve_resize_subject_or_path(right)?;

    if left.descriptor.id.domain() != right.descriptor.id.domain()
        || left.descriptor.id.filter() != right.descriptor.id.filter()
    {
        return Err(BenchError::Config(format!(
            "comp subjects must share domain/filter: {} vs {}",
            left.descriptor.id, right.descriptor.id
        )));
    }

    let fixtures = fixtures_from_flags(&flags)?;
    let scales = scales_from_flags(&flags)?;
    let measurement = MeasurementConfig::from_flags(&flags)?;
    let verification_bounds = verification_bounds_from_flags(&flags)?;
    let runtime_report = tune_runtime(measurement.process_priority());
    log_runtime_tuning(&runtime_report);
    log_preheat_start(measurement.preheat_time());
    preheat_cpu(measurement.preheat_time());
    let mut results = Vec::new();

    for fixture in &fixtures {
        for scale in &scales {
            let output = output_dimensions(fixture.width, fixture.height, scale.x, scale.y);
            let oracle_output = run_resize_once(&left, fixture, output, &ResizeParams::default())?;
            let candidate_output =
                run_resize_once(&right, fixture, output, &ResizeParams::default())?;
            let verification =
                verify_with_bounds(&oracle_output, &candidate_output, verification_bounds);
            if !verification.passed {
                return Err(BenchError::Verify(format!(
                    "{} failed {} verification against {} for {} at {}: {:?}; differing_pixels={}, max_color_distance={:.6}, mean_color_distance={:.6}, rms_color_distance={:.6}",
                    right.descriptor.id,
                    verification.mode,
                    left.descriptor.id,
                    fixture.id,
                    scale.label(),
                    verification.first_mismatch,
                    verification.differing_pixels,
                    verification.max_color_distance,
                    verification.mean_color_distance,
                    verification.rms_color_distance
                )));
            }

            let mut left_observer = ();
            results.push(measure_resize_case(
                &left,
                fixture,
                output,
                *scale,
                &ResizeParams::default(),
                &measurement,
                Some(verification.clone()),
                &mut left_observer,
            )?);
            let mut right_observer = ();
            results.push(measure_resize_case(
                &right,
                fixture,
                output,
                *scale,
                &ResizeParams::default(),
                &measurement,
                Some(verification),
                &mut right_observer,
            )?);
        }
    }

    attach_pair_comparisons(&mut results, &left.descriptor.id, &right.descriptor.id);
    let run = BenchRun::new("comp", domain, Some(measurement.artifact()), results);
    print_comp_table(&run.results, &left.descriptor.id, &right.descriptor.id);
    save_indexed_run(&run, None)?;

    if let Some(out) = flags.optional("--out") {
        write_json(out, &run)?;
    }

    Ok(())
}

pub(crate) fn tile_command(registry: &Registry, args: &[String]) -> Result<(), BenchError> {
    let (domain, rest) = split_domain(args)?;
    if domain != "resize" {
        return Err(BenchError::Config(format!(
            "tile currently supports only resize, got {domain:?}"
        )));
    }

    let flags = Flags::parse(rest)?;
    let subject_id = flags
        .optional("--subject")
        .ok_or_else(|| BenchError::Config("expected --subject".to_owned()))?;
    let subject = registry.resolve_resize_subject_or_path(subject_id)?;

    if !subject.descriptor.capabilities.supports_tiling_params {
        return Err(BenchError::Config(format!(
            "{} does not declare tiling parameter support yet",
            subject.descriptor.id
        )));
    }

    Err(BenchError::Config(
        "tile command surface is present, but no tiled subjects are registered yet".to_owned(),
    ))
}
