//! Baseline and indexed benchmark-run JSON storage under the Cargo target directory.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::{
    error::BenchError,
    result::{
        BenchResult, BenchRun, MeasurementArtifact, ARTIFACT_SCHEMA, ARTIFACT_SCHEMA_VERSION,
    },
    util::checksum,
};

pub(crate) fn save_baseline(
    role: &str,
    name: &str,
    run: &BenchRun,
    replace: bool,
) -> Result<(), BenchError> {
    require_accepted_verification(role, run)?;
    let path = baseline_path(role, name);
    if path.exists() && !replace {
        return Err(BenchError::Baseline(format!(
            "baseline {role}/{name} already exists; use replace to overwrite"
        )));
    }
    write_json(path, &run.as_baseline(role, name))
}

pub(crate) fn load_baseline(role: &str, name: &str) -> Result<BenchRun, BenchError> {
    let path = baseline_path(role, name);
    read_baseline_at(role, name, &path)
}

pub(crate) fn save_scoped_baseline(
    role: &str,
    name: &str,
    run: &BenchRun,
    replace: bool,
) -> Result<(), BenchError> {
    require_accepted_verification(role, run)?;
    for result in &run.results {
        let dir = scoped_baseline_dir(name, run, result);
        if dir.exists() {
            if !replace {
                return Err(BenchError::Baseline(format!(
                    "baseline {role}/{name} already exists for {} {}; use replace to overwrite",
                    result.subject, result.case_id
                )));
            }
            fs::remove_dir_all(&dir).map_err(BenchError::io)?;
        }

        let mut case_run = run.clone();
        case_run.results = vec![result.clone()];
        write_json(
            dir.join(format!("{}.json", sanitize_path_component(&run.run_id))),
            &case_run.as_baseline(role, name),
        )?;
    }
    Ok(())
}

pub(crate) fn load_scoped_baseline(
    role: &str,
    name: &str,
    current_run: &BenchRun,
) -> Result<BenchRun, BenchError> {
    let mut baseline = current_run.as_baseline(role, name);
    baseline.results.clear();

    for result in &current_run.results {
        if let Some(case_baseline) = load_scoped_case_baseline(role, name, current_run, result)? {
            baseline.results.extend(case_baseline.results);
        }
    }

    Ok(baseline)
}

pub(crate) fn load_indexed_run(current_run: &BenchRun) -> Result<Option<BenchRun>, BenchError> {
    let Some(case_runs) = load_indexed_case_runs(current_run, MissingIndexedRun::Skip)? else {
        return Ok(None);
    };
    let mut run = current_run.clone();
    run.results = case_runs
        .into_iter()
        .flat_map(|case_run| case_run.results)
        .collect();
    Ok(Some(run))
}

pub(crate) fn replace_scoped_baseline_from_indexed_run(
    role: &str,
    name: &str,
    current_run: &BenchRun,
) -> Result<usize, BenchError> {
    let case_runs = load_indexed_case_runs(current_run, MissingIndexedRun::Error { role, name })?
        .expect("missing indexed case runs are errors when replacing baselines");

    // Validate every case before replacing any existing accepted artifact.
    for run in &case_runs {
        require_accepted_verification(role, run)?;
    }

    for run in &case_runs {
        let result = run
            .results
            .first()
            .expect("validated indexed case runs contain one result");
        let dir = scoped_baseline_dir(name, run, result);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(BenchError::io)?;
        }
        write_json(
            dir.join(format!("{}.json", sanitize_path_component(&run.run_id))),
            &run.as_baseline(role, name),
        )?;
    }

    Ok(case_runs.len())
}

fn require_accepted_verification(role: &str, run: &BenchRun) -> Result<(), BenchError> {
    if role != "accepted" {
        return Ok(());
    }
    if run.results.is_empty()
        || run.results.iter().any(|result| {
            !result.verified
                || !result.verification.as_ref().is_some_and(|verification| {
                    verification.is_exact()
                        && u64::try_from(verification.pixels).ok()
                            == Some(
                                u64::from(result.output_width) * u64::from(result.output_height),
                            )
                })
        })
    {
        return Err(BenchError::Baseline("accepted baseline requires complete exact verification; numeric bounds and --allow-correctness-failures do not grant Mia's approval".into()));
    }
    Ok(())
}

pub(crate) fn save_indexed_run(
    run: &BenchRun,
    baseline_key: Option<&str>,
) -> Result<(), BenchError> {
    for result in &run.results {
        let path = indexed_result_path(run, result, baseline_key);
        let mut case_run = run.clone();
        case_run.results = vec![result.clone()];
        write_json(path, &case_run)?;
    }
    Ok(())
}

pub(crate) fn write_json(path: impl AsRef<Path>, value: &BenchRun) -> Result<(), BenchError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(BenchError::io)?;
    }
    let data = serde_json::to_string_pretty(value)
        .map_err(|error| BenchError::Runtime(format!("failed to serialize JSON: {error}")))?;
    fs::write(path, data).map_err(BenchError::io)
}

enum MissingIndexedRun<'a> {
    Skip,
    Error { role: &'a str, name: &'a str },
}

fn load_indexed_case_runs(
    current_run: &BenchRun,
    missing: MissingIndexedRun<'_>,
) -> Result<Option<Vec<BenchRun>>, BenchError> {
    let mut case_runs = Vec::with_capacity(current_run.results.len());
    for result in &current_run.results {
        let path = indexed_result_path(current_run, result, None);
        if !path.exists() {
            match missing {
                MissingIndexedRun::Skip => return Ok(None),
                MissingIndexedRun::Error { role, name } => {
                    return Err(BenchError::Baseline(format!(
                        "no latest compatible run for {} {} at {}; run this benchmark profile once before replacing baseline {role}/{name}",
                        result.subject,
                        result.case_id,
                        path.display()
                    )));
                }
            }
        }

        let run = read_json(&path)?;
        validate_artifact(&run, "run", &path)?;
        if run.results.len() != 1 {
            return Err(BenchError::Baseline(format!(
                "indexed run {} contains {} results, expected 1",
                path.display(),
                run.results.len()
            )));
        }
        ensure_indexed_case_matches(&run, current_run, result, &path)?;
        case_runs.push(run);
    }

    Ok(Some(case_runs))
}

fn load_scoped_case_baseline(
    role: &str,
    name: &str,
    current_run: &BenchRun,
    result: &BenchResult,
) -> Result<Option<BenchRun>, BenchError> {
    let dir = scoped_baseline_dir(name, current_run, result);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(BenchError::io(error)),
    };

    let mut newest: Option<(u64, PathBuf, BenchRun)> = None;
    for entry in entries {
        let path = entry.map_err(BenchError::io)?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let run = read_baseline_at(role, name, &path)?;
        if run.results.len() != 1 {
            return Err(BenchError::Baseline(format!(
                "scoped baseline {} contains {} results, expected 1",
                path.display(),
                run.results.len()
            )));
        }
        let candidate = (run.created_at_unix, path, run);
        if newest.as_ref().is_none_or(|(created_at, current_path, _)| {
            candidate.0 > *created_at || (candidate.0 == *created_at && candidate.1 > *current_path)
        }) {
            newest = Some(candidate);
        }
    }

    Ok(newest.map(|(_, _, run)| run))
}

fn read_baseline_at(role: &str, name: &str, path: &Path) -> Result<BenchRun, BenchError> {
    let run = read_json(path)?;
    validate_artifact(&run, "baseline", path)?;
    match run.baseline.as_ref() {
        Some(baseline) if baseline.role == role && baseline.name == name => Ok(run),
        Some(baseline) => Err(BenchError::Baseline(format!(
            "baseline {} has metadata {}/{} but was loaded as {role}/{name}",
            path.display(),
            baseline.role,
            baseline.name
        ))),
        None => Err(BenchError::Baseline(format!(
            "baseline {} is missing baseline metadata",
            path.display()
        ))),
    }
}

fn ensure_indexed_case_matches(
    run: &BenchRun,
    expected_run: &BenchRun,
    expected: &BenchResult,
    path: &Path,
) -> Result<(), BenchError> {
    let actual = run
        .results
        .first()
        .expect("validated indexed case runs contain one result");
    let matches = run.command == expected_run.command
        && run.domain == expected_run.domain
        && run.profile == expected_run.profile
        && run.measurement == expected_run.measurement
        && actual.subject == expected.subject
        && actual.fixture == expected.fixture
        && actual.fixture_fingerprint == expected.fixture_fingerprint
        && actual.filter == expected.filter
        && actual.variant == expected.variant
        && actual.source_width == expected.source_width
        && actual.source_height == expected.source_height
        && actual.output_width == expected.output_width
        && actual.output_height == expected.output_height
        && result_scale_x(actual) == result_scale_x(expected)
        && result_scale_y(actual) == result_scale_y(expected)
        && actual.pixel_format == expected.pixel_format
        && actual.params_fingerprint == expected.params_fingerprint;
    if matches {
        return Ok(());
    }

    Err(BenchError::Baseline(format!(
        "indexed run {} does not match the requested benchmark case {} {}",
        path.display(),
        expected.subject,
        expected.case_id
    )))
}

fn read_json(path: &Path) -> Result<BenchRun, BenchError> {
    let data = fs::read_to_string(path).map_err(|error| {
        BenchError::Baseline(format!(
            "failed to read artifact {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&data).map_err(|error| {
        BenchError::Baseline(format!(
            "failed to parse artifact {}: {error}",
            path.display()
        ))
    })
}

fn validate_artifact(run: &BenchRun, artifact_kind: &str, path: &Path) -> Result<(), BenchError> {
    if run.schema != ARTIFACT_SCHEMA {
        return Err(BenchError::Baseline(format!(
            "artifact {} has schema {:?}, expected {ARTIFACT_SCHEMA:?}",
            path.display(),
            run.schema
        )));
    }
    if run.schema_version > ARTIFACT_SCHEMA_VERSION {
        return Err(BenchError::Baseline(format!(
            "artifact {} has future schema_version {}, expected <= {ARTIFACT_SCHEMA_VERSION}",
            path.display(),
            run.schema_version
        )));
    }
    if run.artifact_kind != artifact_kind {
        return Err(BenchError::Baseline(format!(
            "artifact {} has kind {:?}, expected {artifact_kind:?}",
            path.display(),
            run.artifact_kind
        )));
    }
    Ok(())
}

fn indexed_result_path(
    run: &BenchRun,
    result: &BenchResult,
    baseline_key: Option<&str>,
) -> PathBuf {
    let mut path = artifact_root()
        .join(&run.command)
        .join(&run.domain)
        .join(benchmark_key(result))
        .join(config_key(run, result));
    if let Some(baseline_key) = baseline_key {
        path = path.join(sanitize_path_component(baseline_key));
    }
    path.join(fixture_key(result))
        .join(scale_key(result))
        .join("run.json")
}

fn benchmark_key(result: &BenchResult) -> String {
    compact_component(&result.subject, "unknown-subject")
}

fn config_key(run: &BenchRun, result: &BenchResult) -> String {
    let key = format!(
        "command={}\ndomain={}\nprofile={}\nmeasurement={}\nsubject={}\nfilter={}\nvariant={}\npixel={}\nparams={}\n",
        run.command,
        run.domain,
        run.profile,
        measurement_key(run.measurement.as_ref()),
        result.subject,
        result.filter,
        result.variant,
        result.pixel_format,
        result.params_fingerprint
    );
    let hash = checksum(key.as_bytes());
    format!(
        "{}-{}",
        compact_component(&measurement_key(run.measurement.as_ref()), "default"),
        &hash[..16]
    )
}

fn measurement_key(measurement: Option<&MeasurementArtifact>) -> String {
    let Some(measurement) = measurement else {
        return "no-measurement".to_owned();
    };
    format!(
        "s{}-m{}ms-w{}ms-t{}ms-{}-{}-d{}ms-c{}-l{}-p{}-h{}ms",
        measurement.sample_size,
        measurement.measurement_time_ms,
        measurement.warmup_time_ms,
        measurement.target_sample_time_ms,
        measurement.sample_mode,
        measurement.cache_state,
        measurement.inter_sample_delay_ms,
        measurement.cache_scrub_size,
        measurement.live_stats,
        measurement.process_priority,
        measurement.preheat_time_ms
    )
}

fn fixture_key(result: &BenchResult) -> String {
    compact_component(&result.fixture, "unknown-fixture")
}

fn scale_key(result: &BenchResult) -> String {
    let scale_x = result_scale_x(result);
    let scale_y = result_scale_y(result);
    if (scale_x - scale_y).abs() < f64::EPSILON {
        return format!("{}x", sanitize_path_component(&scale_x.to_string()));
    }

    format!(
        "{}x-{}y",
        sanitize_path_component(&scale_x.to_string()),
        sanitize_path_component(&scale_y.to_string())
    )
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

fn compact_component(value: &str, fallback: &str) -> String {
    let sanitized = sanitize_path_component(value);
    if sanitized.is_empty() {
        return fallback.to_owned();
    }
    if sanitized.len() <= 96 {
        return sanitized;
    }
    let hash = checksum(value.as_bytes());
    format!("{}-{}", &sanitized[..72], &hash[..16])
}

fn sanitize_path_component(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    sanitized.trim_matches('-').to_owned()
}

fn baseline_path(role: &str, name: &str) -> PathBuf {
    artifact_root()
        .join("baselines")
        .join(role)
        .join(format!("{name}.json"))
}

fn scoped_baseline_dir(name: &str, run: &BenchRun, result: &BenchResult) -> PathBuf {
    artifact_root()
        .join("baselines")
        .join(&run.command)
        .join(&run.domain)
        .join(benchmark_key(result))
        .join(config_key(run, result))
        .join(fixture_key(result))
        .join(scale_key(result))
        .join(sanitize_path_component(name))
}

fn artifact_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("bench")
}
