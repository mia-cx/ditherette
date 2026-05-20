//! CLI command implementations.

use ditherette_bench_api::{ResizeBenchSubject, ResizeParams, SubjectId};

use crate::{
    baseline::{
        load_baseline, load_latest_run, save_baseline, save_indexed_run, save_latest_run,
        write_json,
    },
    case::scales_from_flags,
    cli::{split_domain, Flags},
    compare::{
        acceptance_report, attach_accepted_comparisons, attach_measured_oracle_comparisons,
        attach_measured_spec_comparisons, attach_named_oracle_comparisons,
        attach_named_spec_comparisons, attach_pair_comparisons, attach_previous_comparisons,
        ensure_compatible_baseline, has_exact_comparisons, require_accepted_comparisons,
    },
    error::BenchError,
    fixture::fixtures_from_flags,
    measure::{measure_resize_case, run_resize_once, MeasurementConfig},
    registry::Registry,
    report::{
        log_correctness_ok, log_correctness_start, log_perf_start, log_preheat_start,
        log_runtime_tuning, print_comp_table, print_perf_table, MeasurementLogger,
    },
    result::{verify_exact, BenchRun},
    runtime::{preheat_cpu, tune_runtime},
    util::{checksum, normalize_path_string, optional_id, output_dimensions},
};

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
    if let Some(name) = flags.optional("--replace-baseline") {
        let latest = load_latest_run("perf", domain)?;
        save_baseline("accepted", name, &latest, true)?;
        println!("replaced accepted baseline {name:?} from latest perf/{domain} run");
        return Ok(());
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
    let accepted_baseline_name = flags.optional("--baseline");
    let accepted_baseline = accepted_baseline_name
        .map(|name| load_baseline("accepted", name))
        .transpose()?;
    if let Some(baseline) = accepted_baseline.as_ref() {
        ensure_compatible_baseline(baseline, "perf", domain, &measurement)?;
    }
    let previous_run = if accepted_baseline_name.is_none() {
        load_latest_run("perf", domain)
            .ok()
            .filter(|run| ensure_compatible_baseline(run, "perf", domain, &measurement).is_ok())
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
        oracle_id.as_ref(),
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

    if let Some(oracle_id) = oracle_id.as_ref() {
        run_resize_correctness_checks(registry, oracle_id, &subjects, &fixtures, &scales)?;
    }

    let mut results = Vec::new();
    for fixture in &fixtures {
        for scale in &scales {
            let output = output_dimensions(fixture.width, fixture.height, *scale, *scale);
            for subject in &subjects {
                let case = format!("{}-{}x{}-{}x", fixture.id, output.0, output.1, scale);
                let mut logger =
                    MeasurementLogger::new(&subject.descriptor.id.to_string(), &case, measurement);
                let result = measure_resize_case(
                    subject,
                    fixture,
                    output,
                    *scale,
                    &ResizeParams::default(),
                    &measurement,
                    None,
                    &mut logger,
                )?;
                logger.finish(&result);
                results.push(result);
            }
        }
    }

    if let Some(baseline) = accepted_baseline.as_ref() {
        require_accepted_comparisons(&results, baseline)?;
    }
    attach_accepted_comparisons(&mut results, accepted_baseline.as_ref());
    if accepted_baseline.is_none() {
        if let Some(previous_run) = previous_run.as_ref() {
            if has_exact_comparisons(&results, previous_run) {
                attach_previous_comparisons(&mut results, previous_run);
            }
        }
    }
    if let Some(oracle_baseline) = oracle_baseline.as_ref() {
        attach_named_oracle_comparisons(&mut results, oracle_baseline);
    } else if let Some(oracle_id) = oracle_id.as_ref() {
        attach_measured_oracle_comparisons(&mut results, oracle_id);
    }
    if let Some(spec_subject) = measured_spec_subject.as_ref() {
        attach_measured_spec_comparisons(&mut results, spec_subject);
    }
    if let Some(spec_baseline) = loaded_spec_baseline.as_ref() {
        attach_named_spec_comparisons(&mut results, spec_baseline);
    }

    let run = BenchRun::new("perf", domain, Some(measurement.artifact()), results);
    print_perf_table(&run.results);
    let acceptance = acceptance_report(&run.results, &flags)?;

    save_indexed_run(&run, save_baseline_name)?;
    save_latest_run(&run)?;
    if let Some(out) = flags.optional("--out") {
        write_json(out, &run)?;
    }
    if let Some(name) = save_baseline_name {
        save_baseline("accepted", name, &run, true)?;
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

const ORACLE_BASELINE_MAX_AGE_SECONDS: u64 = 24 * 60 * 60;

fn ensure_oracle_baseline(
    oracle: &ResizeBenchSubject,
    subjects: &[ResizeBenchSubject],
    fixtures: &[crate::fixture::Fixture],
    scales: &[f64],
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

    let name = oracle_baseline_name(oracle, fixtures, scales, measurement, domain);
    if !force_refresh {
        if let Ok(baseline) = load_baseline("oracle", &name) {
            if oracle_baseline_is_fresh_and_matching(
                &baseline,
                oracle,
                fixtures,
                scales,
                measurement,
                domain,
            ) {
                return Ok(baseline);
            }
        }
    }

    let reason = if force_refresh {
        "refresh requested"
    } else {
        "missing or stale"
    };
    println!(
        "{} oracle baseline {name:?} {reason}; measuring {}",
        crate::util::heading("Oracle"),
        oracle.descriptor.id
    );
    let mut results = Vec::new();
    for fixture in fixtures {
        for scale in scales {
            let output = output_dimensions(fixture.width, fixture.height, *scale, *scale);
            let case = format!(
                "oracle-baseline/{}-{}x{}-{}x",
                fixture.id, output.0, output.1, scale
            );
            let mut logger =
                MeasurementLogger::new(&oracle.descriptor.id.to_string(), &case, *measurement);
            let result = measure_resize_case(
                oracle,
                fixture,
                output,
                *scale,
                &ResizeParams::default(),
                measurement,
                None,
                &mut logger,
            )?;
            logger.finish(&result);
            results.push(result);
        }
    }
    let run = BenchRun::new("perf", domain, Some(measurement.artifact()), results);
    save_baseline("oracle", &name, &run, true)?;
    load_baseline("oracle", &name)
}

fn oracle_baseline_is_fresh_and_matching(
    baseline: &BenchRun,
    oracle: &ResizeBenchSubject,
    fixtures: &[crate::fixture::Fixture],
    scales: &[f64],
    measurement: &MeasurementConfig,
    domain: &str,
) -> bool {
    ensure_compatible_baseline(baseline, "perf", domain, measurement).is_ok()
        && baseline.created_at_unix + ORACLE_BASELINE_MAX_AGE_SECONDS >= current_unix_seconds()
        && fixtures.iter().all(|fixture| {
            scales.iter().all(|scale| {
                let output = output_dimensions(fixture.width, fixture.height, *scale, *scale);
                baseline.results.iter().any(|result| {
                    result.subject == oracle.descriptor.id.as_str()
                        && result.fixture_fingerprint == fixture.fingerprint
                        && result.source_width == fixture.width
                        && result.source_height == fixture.height
                        && result.output_width == output.0
                        && result.output_height == output.1
                        && result.scale == *scale
                        && result.filter == oracle.descriptor.id.filter()
                        && result.pixel_format == "rgba8"
                        && result.params_fingerprint == "resize-default"
                })
            })
        })
}

fn oracle_baseline_name(
    oracle: &ResizeBenchSubject,
    fixtures: &[crate::fixture::Fixture],
    scales: &[f64],
    measurement: &MeasurementConfig,
    domain: &str,
) -> String {
    let mut key = format!(
        "perf\n{domain}\n{}\n{:?}\n",
        oracle.descriptor.id,
        measurement.artifact()
    );
    for fixture in fixtures {
        for scale in scales {
            let output = output_dimensions(fixture.width, fixture.height, *scale, *scale);
            key.push_str(&format!(
                "{}:{}:{}x{}:{}x{}:{scale}\n",
                fixture.id, fixture.fingerprint, fixture.width, fixture.height, output.0, output.1
            ));
        }
    }
    let hash = checksum(key.as_bytes());
    format!(
        "{}-{}",
        sanitize_name(oracle.descriptor.id.as_str()),
        &hash[..16]
    )
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn run_resize_correctness_checks(
    registry: &Registry,
    oracle_id: &SubjectId,
    subjects: &[ResizeBenchSubject],
    fixtures: &[crate::fixture::Fixture],
    scales: &[f64],
) -> Result<(), BenchError> {
    let oracle = registry.resize_subject(oracle_id.as_str())?;
    let check_count = subjects
        .iter()
        .filter(|subject| subject.descriptor.id != *oracle_id)
        .count()
        * fixtures.len()
        * scales.len();
    log_correctness_start(check_count);

    for fixture in fixtures {
        for scale in scales {
            let output = output_dimensions(fixture.width, fixture.height, *scale, *scale);
            let oracle_output =
                run_resize_once(&oracle, fixture, output, &ResizeParams::default())?;
            let case = format!("{}-{}x{}-{}x", fixture.id, output.0, output.1, scale);

            for subject in subjects {
                if subject.descriptor.id == *oracle_id {
                    continue;
                }
                let candidate_output =
                    run_resize_once(subject, fixture, output, &ResizeParams::default())?;
                let verification = verify_exact(&oracle_output, &candidate_output);
                if !verification.passed {
                    return Err(BenchError::Verify(format!(
                        "{} failed exact verification against {} for {}: {:?}",
                        subject.descriptor.id, oracle_id, case, verification.first_mismatch
                    )));
                }
                log_correctness_ok(&subject.descriptor.id.to_string(), oracle_id, &case);
            }
        }
    }
    println!();
    Ok(())
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
    let runtime_report = tune_runtime(measurement.process_priority());
    log_runtime_tuning(&runtime_report);
    log_preheat_start(measurement.preheat_time());
    preheat_cpu(measurement.preheat_time());
    let mut results = Vec::new();

    for fixture in &fixtures {
        for scale in &scales {
            let output = output_dimensions(fixture.width, fixture.height, *scale, *scale);
            let oracle_output = run_resize_once(&left, fixture, output, &ResizeParams::default())?;
            let candidate_output =
                run_resize_once(&right, fixture, output, &ResizeParams::default())?;
            let verification = verify_exact(&oracle_output, &candidate_output);
            if !verification.passed {
                return Err(BenchError::Verify(format!(
                    "{} failed exact verification against {} for {} at {}x: {:?}",
                    right.descriptor.id,
                    left.descriptor.id,
                    fixture.id,
                    scale,
                    verification.first_mismatch
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
