use ditherette_bench::paired::*;
use ditherette_bench_api::verification::*;

#[path = "fixtures/paired_model.rs"]
mod model;
use model::fixture;

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
    throughput.name = "throughput-native".into();
    throughput.measurement.mode = SampleMode::Throughput;
    throughput.measurement.scope = CallScope::NativeKernel;
    throughput.measurement.application_cache = ApplicationCache::NotApplicable;
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
    // Per-byte indentation dominates actual image requests. Keep that shape in this fixture.
    let dimensions = Dimensions {
        width: 64,
        height: 64,
    };
    template.experiment.cases[0].source = dimensions;
    template.experiment.cases[0].identity.output = dimensions;
    template.experiment.cases[0].rgba = (0..64 * 64 * 4).map(|n| n as u8).collect();
    template.experiment.cases[0].identity.input = input_digest(
        template.experiment.cases[0].source,
        &template.experiment.cases[0].rgba,
    );
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/paired-child.mjs");
    let prepared = coordinator::prepare(
        template.experiment,
        (&script, &"a".repeat(40)),
        (&script, &"b".repeat(40)),
        &directory.join("prepared"),
    )
    .unwrap();
    let read_compact = |path: &Path| {
        let bytes = fs::read(path).unwrap();
        assert!(!bytes.contains(&b'\n'));
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(bytes.len() * 2 < serde_json::to_vec_pretty(&value).unwrap().len());
        value
    };
    assert_eq!(
        read_compact(&directory.join("prepared/prepared.json")),
        serde_json::to_value(&prepared).unwrap()
    );
    let report = coordinator::run(&prepared, &directory.join("success")).unwrap();
    assert_eq!(report.gate, Gate::Pass);
    assert_eq!(
        read_compact(&directory.join("success/prepared.json")),
        serde_json::to_value(&prepared).unwrap()
    );
    assert!(fs::read(directory.join("success/report.json"))
        .unwrap()
        .contains(&b'\n'));
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
    for stem in &starts {
        let value = read_compact(&directory.join(format!("success/{stem}.request.json")));
        assert_eq!(
            value["case"],
            serde_json::to_value(&prepared.experiment.cases[0]).unwrap()
        );
        let request: TrialRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), value);
    }
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
