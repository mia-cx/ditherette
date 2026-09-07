//! Immutable artifact preparation and sequential lease handoff.

use super::*;
use crate::{
    lease::{require_quiet, Lease},
    verification::{content_digest, input_digest, verify_and_preserve},
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const SCHEMA: &str = "ditherette-prepared-pair-v1";

/// Snapshot both already-built artifacts. This function launches no processes.
pub fn prepare(
    experiment: Experiment,
    accepted: (&Path, &str),
    candidate: (&Path, &str),
    directory: &Path,
) -> io::Result<PreparedPair> {
    validate_experiment(&experiment)?;
    for (_, revision) in [accepted, candidate] {
        if ![40, 64].contains(&revision.len()) || !revision.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(io::Error::other("prepare requires full source revisions"));
        }
    }
    // Read both inputs before creating the destination. An absent candidate cannot
    // leave an apparently usable accepted-only preparation.
    let a = fs::read(accepted.0)?;
    let b = fs::read(candidate.0)?;
    if a.is_empty() || b.is_empty() {
        return Err(io::Error::other("empty executable"));
    }
    fs::create_dir(directory)?;
    let directory = fs::canonicalize(directory)?;
    let snapshot = |role: &str, bytes: &[u8], revision: &str| -> io::Result<Executable> {
        let dir = directory.join(role);
        fs::create_dir(&dir)?;
        let path = dir.join("ditherette-bench");
        write_new(&path, bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o555))?;
        }
        Ok(Executable {
            path,
            identity: ArtifactIdentity {
                revision: revision.into(),
                content: content_digest(bytes),
            },
        })
    };
    let prepared = PreparedPair {
        schema: SCHEMA.into(),
        experiment,
        accepted: snapshot("accepted", &a, accepted.1)?,
        candidate: snapshot("candidate", &b, candidate.1)?,
        machine: machine()?,
    };
    write_new(&directory.join("prepared.json"), &json(&prepared)?)?;
    Ok(prepared)
}

/// Run the fixed budget under one lease. No builds or baseline writes occur here.
pub fn run(prepared: &PreparedPair, directory: &Path) -> io::Result<PairReport> {
    require_quiet()?;
    validate_experiment(&prepared.experiment)?;
    if prepared.schema != SCHEMA || prepared.machine != machine()? {
        return Err(io::Error::other(
            "prepared schema or machine identity differs",
        ));
    }
    validate_executable(&prepared.accepted)?;
    validate_executable(&prepared.candidate)?;
    let lease = Lease::exclusive()?;
    fs::create_dir(directory)?;
    let directory = fs::canonicalize(directory)?;
    write_new(&directory.join("prepared.json"), &json(prepared)?)?;
    let mut journal = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("events.jsonl"))?;
    let mut trials = Vec::new();
    for pair in 0..prepared.experiment.pairs {
        let order = if pair % 2 == 0 {
            [Role::Accepted, Role::Candidate]
        } else {
            [Role::Candidate, Role::Accepted]
        };
        for (index, case) in prepared.experiment.cases.iter().enumerate() {
            for role in order {
                let executable = match role {
                    Role::Accepted => &prepared.accepted,
                    Role::Candidate => &prepared.candidate,
                };
                validate_executable(executable)?;
                let stem = format!("pair-{pair:03}-case-{index:03}-{role:?}").to_lowercase();
                let request_path = directory.join(format!("{stem}.request.json"));
                let output_path = directory.join(format!("{stem}.result.json"));
                write_new(
                    &request_path,
                    &json(&TrialRequest {
                        role,
                        pair,
                        reference_state: prepared.experiment.reference_state,
                        executable: executable.identity.clone(),
                        case: case.clone(),
                    })?,
                )?;
                let stdout = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&output_path)?;
                let stderr = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(directory.join(format!("{stem}.stderr")))?;
                let mut command = Command::new(&executable.path);
                command
                    .arg("paired-trial")
                    .arg(&request_path)
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr));
                event(&mut journal, &stem, "starting", None)?;
                let mut child = lease.spawn(command)?;
                event(&mut journal, &stem, "started", Some(child.id()))?;
                let status = child.wait()?;
                event(&mut journal, &stem, "reaped", Some(child.id()))?;
                if !status.success() {
                    return Err(io::Error::other(format!(
                        "{stem} failed ({status}); raw files retained"
                    )));
                }
                validate_executable(executable)?;
                let trial: TrialResult =
                    serde_json::from_slice(&fs::read(&output_path)?).map_err(io::Error::other)?;
                if trial.role != role
                    || trial.pair != pair
                    || trial.case_name != case.name
                    || trial.pid != child.id()
                {
                    return Err(io::Error::other(
                        "child response does not match this invocation",
                    ));
                }
                trials.push(trial);
            }
        }
    }
    // All direct children have exited. Keep the lease while final evidence is written.
    let report = compare(prepared, &trials);
    write_new(&directory.join("report.json"), &json(&report)?)?;
    for (case_index, comparison) in report.cases.iter().enumerate() {
        if comparison.gate != Gate::Incorrect {
            continue;
        }
        let case = &prepared.experiment.cases[case_index];
        for pair in 0..prepared.experiment.pairs {
            let accepted = trials.iter().find(|trial| {
                trial.case_name == case.name && trial.pair == pair && trial.role == Role::Accepted
            });
            let candidate = trials.iter().find(|trial| {
                trial.case_name == case.name && trial.pair == pair && trial.role == Role::Candidate
            });
            if let (Some(a), Some(b)) = (accepted, candidate) {
                for (kind, accepted_record) in
                    [("production", &a.output), ("reference", &b.reference)]
                {
                    let outputs = ThreeWayOutputs {
                        reference_state: prepared.experiment.reference_state,
                        reference: Some(a.reference.clone()),
                        accepted: Some(accepted_record.clone()),
                        candidate: Some(b.output.clone()),
                    };
                    verify_and_preserve(
                        &case.identity,
                        &outputs,
                        VerificationBounds::exact(),
                        &directory.join(format!("review-{case_index:03}-{pair:03}-{kind}")),
                    )?;
                }
            }
        }
    }
    Ok(report)
}

fn validate_executable(executable: &Executable) -> io::Result<()> {
    let metadata = fs::symlink_metadata(&executable.path)?;
    if !metadata.is_file()
        || content_digest(&fs::read(&executable.path)?) != executable.identity.content
    {
        return Err(io::Error::other(
            "prepared executable changed or is not a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o222 != 0 {
            return Err(io::Error::other("prepared executable must be read-only"));
        }
    }
    Ok(())
}

/// Check the experiment at the coordinator and worker input boundaries.
pub fn validate_experiment(experiment: &Experiment) -> io::Result<()> {
    if experiment.label.is_empty()
        || experiment.host_load_notes.is_empty()
        || experiment.cases.is_empty()
        || experiment.pairs < 2
        || experiment.pairs % 2 != 0
    {
        return Err(io::Error::other(
            "experiment needs named cases, host notes, and an even budget of at least two pairs",
        ));
    }
    let mut names = std::collections::BTreeSet::new();
    for case in &experiment.cases {
        if case.name.is_empty() || !names.insert(&case.name) {
            return Err(io::Error::other("empty or duplicate case name"));
        }
        let expected = (u64::from(case.source.width) * u64::from(case.source.height))
            .checked_mul(4)
            .ok_or_else(|| io::Error::other("fixture dimensions overflow byte length"))?;
        if case.source.width == 0
            || case.source.height == 0
            || expected != case.rgba.len() as u64
            || case.identity.input != input_digest(case.source, &case.rgba)
            || case.identity.output.width == 0
            || case.identity.output.height == 0
        {
            return Err(io::Error::other(
                "fixture dimensions or full content digest differ",
            ));
        }
        let m = &case.measurement;
        if m.samples < 5 || m.measurement_ms == 0 || m.warmup_ms == 0 || m.target_sample_ms == 0 {
            return Err(io::Error::other(
                "measurement requires at least five samples and positive timing/warmup settings",
            ));
        }
    }
    Ok(())
}

/// Linux host identity used for this native coordinator. Other platforms fail closed.
pub fn machine() -> io::Result<Machine> {
    #[cfg(target_os = "linux")]
    {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
        Ok(Machine {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            hostname: fs::read_to_string("/proc/sys/kernel/hostname")?
                .trim()
                .into(),
            kernel: fs::read_to_string("/proc/sys/kernel/osrelease")?
                .trim()
                .into(),
            cpu: cpuinfo
                .lines()
                .find_map(|line| {
                    line.strip_prefix("model name")
                        .and_then(|line| line.split_once(':'))
                        .map(|(_, name)| name.trim().to_owned())
                })
                .unwrap_or(cpuinfo),
            logical_cpus: std::thread::available_parallelism()?.get(),
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "native paired host evidence currently requires Linux /proc",
        ))
    }
}

/// Observe live direct benchmark executables; the independent execution lock also enforces exclusion.
pub fn live_benchmarks() -> io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        match fs::read_link(entry.path().join("exe")) {
            Ok(path) => {
                if path
                    .file_name()
                    .is_some_and(|name| name == "ditherette-bench")
                {
                    count += 1;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(count)
}

fn event(file: &mut File, trial: &str, state: &str, pid: Option<u32>) -> io::Result<()> {
    #[derive(Serialize)]
    struct Event<'a> {
        trial: &'a str,
        state: &'a str,
        pid: Option<u32>,
        unix_ns: u128,
        host_load: String,
    }
    let entry = Event {
        trial,
        state,
        pid,
        unix_ns: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos(),
        host_load: fs::read_to_string("/proc/loadavg")?.trim().into(),
    };
    serde_json::to_writer(&mut *file, &entry).map_err(io::Error::other)?;
    file.write_all(b"\n")?;
    file.flush()
}

fn json(value: &impl Serialize) -> io::Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(io::Error::other)
}
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(bytes)
}
