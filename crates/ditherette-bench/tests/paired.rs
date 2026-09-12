use ditherette_bench::{paired::*, verification::content_digest};
use ditherette_bench_api::verification::*;

fn fixture() -> (PreparedPair, Vec<TrialResult>) {
    let artifact = ArtifactIdentity {
        revision: "a".repeat(40),
        content: content_digest(b"executable"),
    };
    let measurement = Measurement {
        mode: SampleMode::SingleCall,
        scope: CallScope::NativeKernel,
        application_cache: ApplicationCache::NotApplicable,
        samples: 5,
        measurement_ms: 10,
        warmup_ms: 1,
        target_sample_ms: 1,
    };
    let identity = CaseIdentity {
        semantics: SemanticIdentity {
            operation: Operation::Resize,
            recipe: "nearest-center-default".into(),
            version: 1,
            space: None,
        },
        input: content_digest(b"input"),
        settings: content_digest(b"settings"),
        output: Dimensions {
            width: 1,
            height: 1,
        },
    };
    let subject = "spec:resize:nearest:scalar".to_owned();
    let case = PairCase {
        name: "one-call".into(),
        identity: identity.clone(),
        source: identity.output,
        rgba: vec![1, 2, 3, 255],
        reference_subject: subject.clone(),
        accepted_subject: subject.clone(),
        candidate_subject: subject.clone(),
        measurement: measurement.clone(),
    };
    let prepared = PreparedPair {
        schema: "ditherette-prepared-pair-v1".into(),
        experiment: Experiment {
            label: "control fixture".into(),
            reference_state: ReferenceState::PreFreeze,
            pairs: 2,
            host_load_notes: "deterministic fake samples".into(),
            cases: vec![case],
        },
        accepted: Executable {
            path: "accepted/ditherette-bench".into(),
            identity: artifact.clone(),
        },
        candidate: Executable {
            path: "candidate/ditherette-bench".into(),
            identity: ArtifactIdentity {
                revision: "b".repeat(40),
                content: content_digest(b"candidate executable"),
            },
        },
        machine: Machine {
            os: "fixture".into(),
            arch: "fixture".into(),
            hostname: "fixture".into(),
            kernel: "fixture".into(),
            cpu: "fixture".into(),
            logical_cpus: 1,
        },
    };
    let record = RecordedOutput {
        case: identity,
        implementation: ImplementationIdentity { subject, artifact },
        output: VerificationOutput {
            dimensions: Dimensions {
                width: 1,
                height: 1,
            },
            pixels: Pixels::Rgba8 {
                data: vec![1, 2, 3, 255],
            },
            warnings: Vec::new(),
        },
    };
    let mut trials = Vec::new();
    for pair in 0..2 {
        for role in [Role::Accepted, Role::Candidate] {
            let mut record = record.clone();
            record.implementation.artifact = match role {
                Role::Accepted => prepared.accepted.identity.clone(),
                Role::Candidate => prepared.candidate.identity.clone(),
            };
            trials.push(TrialResult {
                pair,
                role,
                case_name: "one-call".into(),
                build: BuildIdentity {
                    revision: record.implementation.artifact.revision.clone(),
                    dirty: false,
                    rustc: "rustc fixture".into(),
                    tool_version: "fixture".into(),
                    configuration: "fixture build configuration".into(),
                },
                measurement: measurement.clone(),
                warmup_iterations: 1,
                warmup_elapsed_ns: 1,
                sample_ns: vec![100.0; 5],
                iterations_per_sample: 1,
                reference: record.clone(),
                output: record.clone(),
                pid: 123,
                max_live_benchmark_processes: 1,
            });
        }
    }
    (prepared, trials)
}

#[test]
fn fresh_pairs_confirm_per_case_regressions_without_promoting_code() {
    let (prepared, mut trials) = fixture();
    let original = prepared.accepted.identity.clone();
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    for trial in &mut trials {
        if trial.role == Role::Candidate {
            trial.sample_ns.fill(110.0);
        }
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    for trial in &mut trials {
        if trial.role == Role::Candidate {
            trial.sample_ns.fill(111.0);
        }
    }
    let report = compare(&prepared, &trials);
    assert_eq!(report.gate, Gate::Regression);
    assert_eq!(report.cases[0].pair_ratios, vec![1.11, 1.11]);
    assert_eq!(prepared.accepted.identity, original);
    trials[3].sample_ns.fill(99.0);
    assert_eq!(compare(&prepared, &trials).gate, Gate::Inconclusive);
}

#[test]
fn missing_identity_settings_samples_or_correctness_never_pass() {
    let (prepared, trials) = fixture();
    let mut malformed = prepared.experiment.clone();
    malformed.cases[0].source = Dimensions {
        width: u32::MAX,
        height: u32::MAX,
    };
    assert!(coordinator::validate_experiment(&malformed)
        .unwrap_err()
        .to_string()
        .contains("overflow"));
    assert_eq!(compare(&prepared, &trials[..3]).gate, Gate::Incomplete);
    let mut duplicate = trials.clone();
    duplicate.push(trials[0].clone());
    assert_eq!(compare(&prepared, &duplicate).gate, Gate::Incomplete);
    for mutation in 0..6 {
        let mut changed = trials.clone();
        match mutation {
            0 => changed[0].build.revision = "b".repeat(40),
            1 => changed[0].measurement.application_cache = ApplicationCache::Cold,
            2 => changed[0].sample_ns[0] = f64::NAN,
            3 => changed[0].iterations_per_sample = 2,
            4 => changed[0].max_live_benchmark_processes = 2,
            _ => changed[0].build.rustc = "different compiler".into(),
        }
        assert_eq!(compare(&prepared, &changed).gate, Gate::Incomplete);
    }
    let mut incorrect = trials;
    if let Pixels::Rgba8 { data } = &mut incorrect[3].output.output.pixels {
        data[0] += 1;
    }
    assert_eq!(compare(&prepared, &incorrect).gate, Gate::Incorrect);
}

#[test]
fn latency_throughput_and_app_cache_cases_remain_separate() {
    let (mut prepared, mut trials) = fixture();
    let mut throughput = prepared.experiment.cases[0].clone();
    throughput.name = "throughput-warm".into();
    throughput.measurement.mode = SampleMode::Throughput;
    throughput.measurement.scope = CallScope::CompleteCall;
    throughput.measurement.application_cache = ApplicationCache::Warm;
    prepared.experiment.cases.push(throughput.clone());
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    let extra: Vec<_> = trials
        .iter()
        .cloned()
        .map(|mut trial| {
            trial.case_name = throughput.name.clone();
            trial.measurement = throughput.measurement.clone();
            trial.iterations_per_sample = 10;
            trial.sample_ns.fill(20.0);
            trial
        })
        .collect();
    trials.extend(extra);
    let report = compare(&prepared, &trials);
    assert_eq!(report.gate, Gate::Pass);
    assert_eq!(report.cases[0].accepted_median_ns, Some(100.0));
    assert_eq!(report.cases[1].accepted_median_ns, Some(20.0));
}

#[test]
#[cfg(target_os = "linux")]
fn coordinator_holds_one_lease_across_alternating_children_and_failures() {
    use ditherette_bench::{
        lease::{Lease, QUIET_ENV},
        paired::coordinator,
        verification::input_digest,
    };
    use std::{
        fs,
        path::Path,
        time::{SystemTime, UNIX_EPOCH},
    };
    let directory = std::env::temp_dir().join(format!(
        "ditherette-pair-fixture-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    std::env::set_var(QUIET_ENV, "1");
    std::env::set_var("DITHERETTE_PAIR_FIXTURE_DIRECTORY", &directory);
    let (mut template, _) = fixture();
    template.experiment.cases[0].identity.input = input_digest(
        template.experiment.cases[0].source,
        &template.experiment.cases[0].rgba,
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/paired-child.mjs");
    assert!(coordinator::prepare(
        template.experiment.clone(),
        (&script, &"a".repeat(40)),
        (&script, &"a".repeat(40)),
        &directory.join("identical"),
    )
    .unwrap_err()
    .to_string()
    .contains("distinct"));
    assert!(!directory.join("identical").exists());
    let prepared = coordinator::prepare(
        template.experiment,
        (&script, &"a".repeat(40)),
        (&script, &"b".repeat(40)),
        &directory.join("prepared"),
    )
    .unwrap();
    let mut same_revision = prepared.clone();
    same_revision.candidate.identity.revision = prepared.accepted.identity.revision.clone();
    assert!(
        coordinator::run(&same_revision, &directory.join("edited-identical"))
            .unwrap_err()
            .to_string()
            .contains("distinct")
    );
    assert!(!directory.join("edited-identical").exists());
    let report = coordinator::run(&prepared, &directory.join("success")).unwrap();
    assert_eq!(report.gate, Gate::Pass);
    let events = fs::read_to_string(directory.join("success/events.jsonl")).unwrap();
    let lines: Vec<serde_json::Value> = events
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let starts: Vec<_> = lines
        .iter()
        .filter(|line| line["state"] == "started")
        .map(|line| line["trial"].as_str().unwrap())
        .collect();
    assert_eq!(
        starts,
        [
            "pair-000-case-000-accepted",
            "pair-000-case-000-candidate",
            "pair-001-case-000-candidate",
            "pair-001-case-000-accepted"
        ]
    );
    for group in lines.chunks_exact(3) {
        assert_eq!(group[2]["state"], "reaped");
        assert_eq!(group[1]["pid"], group[2]["pid"]);
    }
    assert!(!directory.join("active").exists());
    assert!(coordinator::run(&prepared, &directory.join("success")).is_err());
    for failure in ["exit", "json"] {
        std::env::set_var("DITHERETTE_PAIR_FIXTURE_FAILURE", failure);
        assert!(coordinator::run(&prepared, &directory.join(failure)).is_err());
        assert!(!directory.join("active").exists());
        drop(Lease::exclusive().unwrap());
    }
    std::env::set_var("DITHERETTE_PAIR_FIXTURE_FAILURE", "reference");
    assert_eq!(
        coordinator::run(&prepared, &directory.join("reference"))
            .unwrap()
            .gate,
        Gate::Incorrect
    );
    assert!(directory
        .join("reference/review-000-000-reference/accepted.png")
        .exists());
    assert!(!directory
        .join("reference/review-000-000-production")
        .exists());
    std::env::set_var("DITHERETTE_PAIR_FIXTURE_FAILURE", "mixed");
    assert_eq!(
        coordinator::run(&prepared, &directory.join("mixed"))
            .unwrap()
            .gate,
        Gate::Incorrect
    );
    assert!(directory
        .join("mixed/review-000-000-reference/accepted.png")
        .exists());
    std::env::remove_var("DITHERETTE_PAIR_FIXTURE_FAILURE");
    // A permission change or byte replacement fails before another child starts.
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&prepared.candidate.path, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(coordinator::run(&prepared, &directory.join("mutable")).is_err());
    fs::write(&prepared.candidate.path, b"replacement").unwrap();
    fs::set_permissions(&prepared.candidate.path, fs::Permissions::from_mode(0o555)).unwrap();
    assert!(coordinator::run(&prepared, &directory.join("changed")).is_err());
    std::env::remove_var(QUIET_ENV);
    std::env::remove_var("DITHERETTE_PAIR_FIXTURE_DIRECTORY");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn copied_revisions_cannot_form_a_cross_revision_gate() {
    let (mut prepared, mut trials) = fixture();
    prepared.candidate.identity = prepared.accepted.identity.clone();
    for trial in &mut trials {
        if trial.role == Role::Candidate {
            trial.build.revision = prepared.accepted.identity.revision.clone();
            trial.output.implementation.artifact = prepared.accepted.identity.clone();
            trial.reference.implementation.artifact = prepared.accepted.identity.clone();
        }
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
}

#[test]
fn output_byte_lengths_are_checked_before_native_allocation() {
    let (mut prepared, _) = fixture();
    let case = &mut prepared.experiment.cases[0];
    case.identity.input = ditherette_bench::verification::input_digest(case.source, &case.rgba);
    case.identity.output = Dimensions {
        width: u32::MAX,
        height: u32::MAX,
    };
    assert!(coordinator::validate_experiment(&prepared.experiment)
        .unwrap_err()
        .to_string()
        .contains("output dimensions overflow"));
}

#[test]
fn known_correctness_failures_survive_incomplete_timing_evidence() {
    let (mut prepared, mut trials) = fixture();
    if let Pixels::Rgba8 { data } = &mut trials[1].output.output.pixels {
        data[0] += 1;
    }
    trials[3].sample_ns.clear();
    let report = compare(&prepared, &trials);
    assert_eq!(report.cases[0].gate, Gate::Incorrect);
    assert_eq!(report.gate, Gate::Incorrect);
    let mut missing = prepared.experiment.cases[0].clone();
    missing.name = "missing-case".into();
    prepared.experiment.cases.push(missing);
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incorrect);
}

#[test]
fn compilation_settings_must_match_between_roles() {
    let (prepared, trials) = fixture();
    for configuration in [
        "",
        "different target",
        "different profile",
        "different opt level",
        "different debug info",
        "different features",
        "different codegen flags",
    ] {
        let mut changed = trials.clone();
        changed[1].build.configuration = configuration.into();
        assert_eq!(compare(&prepared, &changed).gate, Gate::Incomplete);
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
}
