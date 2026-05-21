//! Baseline and latest-run JSON storage under the Cargo target directory.

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
    let path = scoped_baseline_path(role, name, run);
    if path.exists() && !replace {
        return Err(BenchError::Baseline(format!(
            "baseline {role}/{name} already exists for this config; use replace to overwrite"
        )));
    }
    write_json(path, &run.as_baseline(role, name))
}

pub(crate) fn load_scoped_baselines(role: &str, name: &str) -> Result<Vec<BenchRun>, BenchError> {
    let mut baselines = Vec::new();
    let legacy_path = baseline_path(role, name);
    if legacy_path.exists() {
        baselines.push(read_baseline_at(role, name, &legacy_path)?);
    }

    let dir = scoped_baseline_dir(role, name);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(baselines),
        Err(error) => return Err(BenchError::io(error)),
    };
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()).map_err(BenchError::io))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    for path in paths {
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        baselines.push(read_baseline_at(role, name, &path)?);
    }

    Ok(baselines)
}

pub(crate) fn save_latest_run(run: &BenchRun) -> Result<(), BenchError> {
    write_json(latest_run_path(&run.command, &run.domain), run)
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

pub(crate) fn load_latest_run(command: &str, domain: &str) -> Result<BenchRun, BenchError> {
    let path = latest_run_path(command, domain);
    let run = read_json(&path).map_err(|error| match error {
        BenchError::Baseline(message) => BenchError::Baseline(format!(
            "{message}; run the benchmark once before replacing a baseline"
        )),
        error => error,
    })?;
    validate_artifact(&run, "run", &path)?;
    if run.command != command || run.domain != domain {
        return Err(BenchError::Baseline(format!(
            "latest run {} is for {}/{} but expected {command}/{domain}",
            path.display(),
            run.command,
            run.domain
        )));
    }
    Ok(run)
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
    path.join(sanitize_path_component(&result.fixture))
        .join(scale_key(result.scale))
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

fn scale_key(scale: f64) -> String {
    format!("{}x", sanitize_path_component(&scale.to_string()))
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

fn latest_run_path(command: &str, domain: &str) -> PathBuf {
    artifact_root()
        .join("latest")
        .join(format!("{command}-{domain}.json"))
}

fn baseline_path(role: &str, name: &str) -> PathBuf {
    artifact_root()
        .join("baselines")
        .join(role)
        .join(format!("{name}.json"))
}

fn scoped_baseline_path(role: &str, name: &str, run: &BenchRun) -> PathBuf {
    scoped_baseline_dir(role, name).join(format!("{}.json", scoped_baseline_key(run)))
}

fn scoped_baseline_dir(role: &str, name: &str) -> PathBuf {
    artifact_root()
        .join("baselines")
        .join(role)
        .join(sanitize_path_component(name))
}

fn scoped_baseline_key(run: &BenchRun) -> String {
    let mut result_keys = run
        .results
        .iter()
        .map(|result| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                result.subject,
                result.variant,
                result.fixture_fingerprint,
                result.source_width,
                result.source_height,
                result.output_width,
                result.output_height,
                result.scale,
                result.filter,
                result.pixel_format,
                result.params_fingerprint,
                result.fixture
            )
        })
        .collect::<Vec<_>>();
    result_keys.sort();
    let key = format!(
        "command={}\ndomain={}\nprofile={}\nmeasurement={:?}\nresults={}\n",
        run.command,
        run.domain,
        run.profile,
        run.measurement,
        result_keys.join("\n")
    );
    checksum(key.as_bytes())
}

fn artifact_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("bench")
}
