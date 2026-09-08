use ditherette_bench::{
    paired::{browser::*, *},
    verification::content_digest,
};
use ditherette_bench_api::verification::*;

#[path = "fixtures/paired_model.rs"]
mod model;

#[test]
fn javascript_warm_trial_payload_preserves_prime_evidence_in_strict_transport() {
    let script = r#"
import { warmProcessTrial } from './scripts/benchmark-stage-trial-fixture.mjs';
import { attachOracleReference } from './scripts/benchmark-public-browser.mjs';
const { result, trial } = await warmProcessTrial();
const oracle = JSON.parse(process.env.ORACLE_FIXTURE);
attachOracleReference(result, { ...oracle, prime_output: trial.prime_reference_output });
Object.assign(result.observation, {
    engine: 'chromium', browser_version: 'fixture', node_version: process.version,
    playwright_version: 'fixture'
});
console.log(JSON.stringify(result));
"#;
    let (_, trials) = fixture();
    let oracle = OracleOutput {
        case: trials[0].reference.case.clone(),
        output: trials[0].reference.output.clone(),
    };
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "-e", script])
        .env("ORACLE_FIXTURE", serde_json::to_string(&oracle).unwrap())
        .current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        payload["prime_reference_output"]["pixels"]["data"],
        serde_json::json!([1, 2, 3, 255])
    );
    let parsed: BrowserTransportResult = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.sample_ns.len(), 5);
    let encoded = serde_json::to_value(parsed).unwrap();
    assert_eq!(
        encoded["prime_reference_output"],
        payload["prime_reference_output"]
    );
}

#[test]
fn trilinear_binds_anchors_and_rejects_nonexistent_website_operation() {
    let (mut prepared, _) = fixture();
    let case = &mut prepared.experiment.cases[0];
    let operation = PublicOperation::ResizeTrilinear {
        anchor: Anchor::Left,
    };
    let browser = case.browser.as_mut().unwrap();
    browser.operation = operation.clone();
    browser.accepted = BrowserBackend::Package;
    case.reference_subject = operation.reference_subject().into();
    case.accepted_subject = operation.subject(browser.accepted).into();
    case.candidate_subject = operation.subject(browser.candidate).into();
    case.identity = operation
        .identity(case.source, &case.rgba, case.identity.output)
        .unwrap();
    validate_case(case).unwrap();
    let center = PublicOperation::ResizeTrilinear {
        anchor: Anchor::Center,
    };
    assert_ne!(
        case.identity,
        center
            .identity(case.source, &case.rgba, case.identity.output)
            .unwrap()
    );
    case.browser.as_mut().unwrap().accepted = BrowserBackend::TypeScript;
    case.accepted_subject = operation.subject(BrowserBackend::TypeScript).into();
    assert!(validate_case(case)
        .unwrap_err()
        .to_string()
        .contains("no bicubic or trilinear"));
}

#[test]
fn area_and_bilinear_keep_distinct_recipes_and_validate_website_anchors() {
    for (operation, filter) in [
        (PublicOperation::ResizeArea {}, "area"),
        (
            PublicOperation::ResizeBilinear {
                anchor: Anchor::Center,
            },
            "bilinear",
        ),
    ] {
        let (mut prepared, _) = fixture();
        let case = &mut prepared.experiment.cases[0];
        let browser = case.browser.as_mut().unwrap();
        browser.operation = operation.clone();
        case.reference_subject = operation.reference_subject().into();
        case.accepted_subject = operation.subject(browser.accepted).into();
        case.candidate_subject = operation.subject(browser.candidate).into();
        case.identity = operation
            .identity(case.source, &case.rgba, case.identity.output)
            .unwrap();
        assert_eq!(
            case.reference_subject,
            format!("spec:resize:{filter}:scalar")
        );
        assert_eq!(
            case.identity.semantics.recipe,
            format!("{filter}-public-v1")
        );
        validate_case(case).unwrap();
        let round_trip: PublicOperation =
            serde_json::from_str(&serde_json::to_string(&operation).unwrap()).unwrap();
        assert_eq!(round_trip, operation);
        case.browser.as_mut().unwrap().operation = PublicOperation::ResizeBilinear {
            anchor: Anchor::Left,
        };
        assert!(validate_case(case).is_err());
    }
    assert!(serde_json::from_str::<PublicOperation>(
        r#"{"operation":"resize-area","anchor":"center"}"#
    )
    .is_err());
}

fn fixture() -> (PreparedPair, Vec<TrialResult>) {
    let (mut prepared, mut trials) = model::fixture();
    let browser = BrowserCase {
        execution: None,
        operation: PublicOperation::ResizeNearest {
            anchor: Anchor::Center,
        },
        accepted: BrowserBackend::TypeScript,
        candidate: BrowserBackend::Package,
        preparation: BrowserPreparation::PrimedInstance,
        cache: CacheCapability::None,
        measure_nonexact: false,
        progress: None,
        threads: None,
    };
    let case = &mut prepared.experiment.cases[0];
    case.identity = browser
        .operation
        .identity(case.source, &case.rgba, case.identity.output)
        .unwrap();
    case.reference_subject = browser.operation.reference_subject().into();
    case.accepted_subject = browser.operation.subject(browser.accepted).into();
    case.candidate_subject = browser.operation.subject(browser.candidate).into();
    case.measurement.scope = CallScope::CompleteCall;
    case.browser = Some(browser.clone());
    let files: Vec<_> = [
        "package.js",
        "page.html",
        "playwright.js",
        "transport.mjs",
        "typescript.js",
        "wasm.wasm",
    ]
    .into_iter()
    .map(|path| AssetFile {
        alias_of: None,
        path: path.into(),
        bytes: 1,
        mode: 0o444,
        digest: content_digest(path.as_bytes()),
    })
    .collect();
    let tree = AssetTree {
        root: "/fixture/assets".into(),
        digest: tree_digest(&files).unwrap(),
        files,
    };
    let assets = AssetBundle {
        tree: tree.clone(),
        entries: AssetEntries {
            package: "package.js".into(),
            typescript: "typescript.js".into(),
            transport: "transport.mjs".into(),
            page: "page.html".into(),
            wasm: "wasm.wasm".into(),
        },
    };
    let browser_files = vec![AssetFile {
        alias_of: None,
        path: "browser".into(),
        bytes: 7,
        mode: 0o555,
        digest: content_digest(b"browser"),
    }];
    let runtime = BrowserRuntime {
        engine: BrowserEngine::Chromium,
        node: RuntimeBinary {
            path: "/fixture/node".into(),
            digest: content_digest(b"node"),
            version: "node-fixture".into(),
        },
        browser: RuntimeBinary {
            path: "/fixture/browser".into(),
            digest: content_digest(b"browser"),
            version: "browser-fixture".into(),
        },
        browser_assets: AssetTree {
            root: "/fixture".into(),
            digest: tree_digest(&browser_files).unwrap(),
            files: browser_files,
        },
        playwright: RuntimePackage {
            tree,
            entry: "playwright.js".into(),
            version: "playwright-fixture".into(),
        },
        launch_args: vec!["--fixture".into()],
        headless: true,
        cross_origin_isolated: true,
    };
    prepared.browser = Some(PreparedBrowser {
        accepted: assets.clone(),
        candidate: assets.clone(),
        runtime: runtime.clone(),
    });
    for trial in &mut trials {
        let worker = match trial.role {
            Role::Accepted => &prepared.accepted,
            Role::Candidate => &prepared.candidate,
        };
        trial.measurement = case.measurement.clone();
        trial.sample_ns.fill(1000.0);
        trial.output.case = case.identity.clone();
        trial.reference.case = case.identity.clone();
        trial.output.implementation.subject = browser
            .operation
            .subject(browser.backend(trial.role))
            .into();
        trial.reference.implementation.subject = browser.operation.reference_subject().into();
        trial.output.implementation.artifact =
            artifact_identity(&worker.identity, &assets, &runtime).unwrap();
        trial.reference.implementation.artifact = trial.output.implementation.artifact.clone();
        trial.browser = Some(BrowserEvidence {
            measure_nonexact: false,
            progress: None,
            threads: None,
            assets: assets.tree.digest,
            runtime: runtime_digest(&runtime).unwrap(),
            backend: browser.backend(trial.role),
            preparation: browser.preparation,
            cache: browser.cache,
            observation: BrowserObservation {
                execution: None,
                engine: runtime.engine,
                browser_version: runtime.browser.version.clone(),
                node_version: runtime.node.version.clone(),
                playwright_version: runtime.playwright.version.clone(),
                user_agent: "fixture-UA".into(),
                cross_origin_isolated: true,
                timer_resolution_ns: 1.0,
            },
        });
    }
    (prepared, trials)
}

#[test]
fn convolution_support_binds_reference_and_recipe_identity() {
    for support in [Support::Fixed, Support::ScaleAware] {
        for operation in [
            PublicOperation::ResizeBicubic {
                anchor: Anchor::Center,
                support,
            },
            PublicOperation::ResizeLanczos2 {
                anchor: Anchor::Center,
                support,
            },
            PublicOperation::ResizeLanczos3 {
                anchor: Anchor::Center,
                support,
            },
        ] {
            let (mut prepared, _) = fixture();
            let case = &mut prepared.experiment.cases[0];
            let browser = case.browser.as_mut().unwrap();
            browser.operation = operation.clone();
            if matches!(operation, PublicOperation::ResizeBicubic { .. }) {
                browser.accepted = BrowserBackend::Package;
            }
            case.reference_subject = operation.reference_subject().into();
            case.accepted_subject = operation.subject(browser.accepted).into();
            case.candidate_subject = operation.subject(browser.candidate).into();
            case.identity = operation
                .identity(case.source, &case.rgba, case.identity.output)
                .unwrap();
            assert_eq!(
                case.reference_subject.ends_with("scale-aware"),
                support == Support::ScaleAware
            );
            validate_case(case).unwrap();
            if matches!(operation, PublicOperation::ResizeBicubic { .. }) {
                case.browser.as_mut().unwrap().accepted = BrowserBackend::TypeScript;
                assert!(validate_case(case).is_err());
            }
        }
    }
    let source = Dimensions {
        width: 3,
        height: 2,
    };
    let rgba = vec![255; 24];
    let fixed = PublicOperation::ResizeLanczos2 {
        anchor: Anchor::Center,
        support: Support::Fixed,
    };
    let scaled = PublicOperation::ResizeLanczos2 {
        anchor: Anchor::Center,
        support: Support::ScaleAware,
    };
    assert_ne!(
        fixed.identity(source, &rgba, source).unwrap().settings,
        scaled.identity(source, &rgba, source).unwrap().settings
    );
}

#[test]
fn typed_browser_calls_share_exact_three_way_gates() {
    let (prepared, mut trials) = fixture();
    coordinator::validate_experiment(&prepared.experiment).unwrap();
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    for trial in &mut trials {
        if trial.role == Role::Candidate {
            trial.sample_ns.fill(1120.0);
        }
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Regression);
    if let Pixels::Rgba8 { data } = &mut trials[0].output.output.pixels {
        data[0] += 1;
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incorrect);
}

#[test]
fn progress_roles_preserve_historical_json_and_bind_trial_evidence() {
    let (mut prepared, mut trials) = fixture();
    let case = &mut prepared.experiment.cases[0];
    let browser = case.browser.as_mut().unwrap();
    let historical = serde_json::to_value(&*browser).unwrap();
    assert!(historical.get("progress").is_none());
    let decoded: BrowserCase = serde_json::from_value(historical.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), historical);
    let roles = ProgressRoles {
        accepted: ProgressMode::Disabled,
        candidate: ProgressMode::Enabled,
    };
    browser.progress = Some(roles);
    browser.accepted = BrowserBackend::Package;
    browser.preparation = BrowserPreparation::FreshInstance;
    browser.cache = CacheCapability::Roles {
        accepted: PreparationCapability::ImageStages,
        candidate: PreparationCapability::ImageStages,
        sample_prime: None,
    };
    case.accepted_subject = browser.operation.subject(BrowserBackend::Package).into();
    case.measurement.mode = SampleMode::SingleCall;
    case.measurement.application_cache = ApplicationCache::Cold;
    validate_case(case).unwrap();
    for trial in &mut trials {
        trial.measurement = case.measurement.clone();
        trial.output.implementation.subject = case.accepted_subject.clone();
        let evidence = trial.browser.as_mut().unwrap();
        let historical = serde_json::to_value(&*evidence).unwrap();
        assert!(historical.get("progress").is_none());
        let decoded: BrowserEvidence = serde_json::from_value(historical.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), historical);
        evidence.backend = BrowserBackend::Package;
        evidence.preparation = BrowserPreparation::FreshInstance;
        evidence.cache = case.browser.as_ref().unwrap().cache;
        evidence.progress = Some(roles);
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    trials[0].browser.as_mut().unwrap().progress = None;
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    let case = &mut prepared.experiment.cases[0];
    case.measurement.application_cache = ApplicationCache::Warm;
    assert!(validate_case(case).is_err());
}

#[test]
fn thread_roles_preserve_historical_json_and_bind_initialization_evidence() {
    let (mut prepared, mut trials) = fixture();
    let case = &mut prepared.experiment.cases[0];
    let browser = case.browser.as_mut().unwrap();
    let historical = serde_json::to_value(&*browser).unwrap();
    assert!(historical.get("threads").is_none());
    assert!(historical.get("execution").is_none());
    let decoded: BrowserCase = serde_json::from_value(historical.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), historical);
    let roles = ThreadRoles {
        accepted: Threads::Disabled,
        candidate: Threads::Required,
    };
    browser.threads = Some(roles);
    browser.execution = Some(BrowserExecution::HostWorker);
    browser.accepted = BrowserBackend::Package;
    browser.preparation = BrowserPreparation::InitializationCompiled;
    case.accepted_subject = browser.operation.subject(BrowserBackend::Package).into();
    case.measurement.mode = SampleMode::SingleCall;
    case.measurement.scope = CallScope::Initialization;
    validate_case(case).unwrap();
    let mut unsupported = case.clone();
    unsupported.measurement.scope = CallScope::CompleteCall;
    assert!(validate_case(&unsupported).is_err());
    for trial in &mut trials {
        trial.measurement = case.measurement.clone();
        trial.output.implementation.subject = case.accepted_subject.clone();
        let evidence = trial.browser.as_mut().unwrap();
        let historical = serde_json::to_value(&*evidence).unwrap();
        assert!(historical.get("threads").is_none());
        let decoded: BrowserEvidence = serde_json::from_value(historical.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), historical);
        evidence.backend = BrowserBackend::Package;
        evidence.preparation = BrowserPreparation::InitializationCompiled;
        evidence.threads = Some(roles);
        evidence.observation.execution = Some(BrowserExecution::HostWorker);
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    trials[0].browser.as_mut().unwrap().observation.execution = None;
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    trials[0].browser.as_mut().unwrap().observation.execution = Some(BrowserExecution::Page);
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    trials[0].browser.as_mut().unwrap().observation.execution = Some(BrowserExecution::HostWorker);
    trials[0].browser.as_mut().unwrap().threads = None;
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    trials[0].browser.as_mut().unwrap().threads = Some(ThreadRoles {
        accepted: Threads::Disabled,
        candidate: Threads::Preferred,
    });
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    let case = &mut prepared.experiment.cases[0];
    case.browser.as_mut().unwrap().accepted = BrowserBackend::TypeScript;
    assert!(validate_case(case).is_err());
}

#[test]
fn timing_nonexact_outputs_never_passes_conformance_and_binds_the_diagnostic_flag() {
    let (mut prepared, mut trials) = fixture();
    prepared.experiment.cases[0]
        .browser
        .as_mut()
        .unwrap()
        .measure_nonexact = true;
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    for trial in &mut trials {
        trial.browser.as_mut().unwrap().measure_nonexact = true;
        if trial.role == Role::Candidate {
            if let Pixels::Rgba8 { data } = &mut trial.output.output.pixels {
                data[0] += 1;
            }
        }
    }
    let report = compare(&prepared, &trials);
    assert_eq!(report.gate, Gate::Incorrect);
    assert!(report.cases[0].accepted_median_ns.is_some());
    assert!(report.cases[0].candidate_median_ns.is_some());
    assert!(!report.cases[0].verification[0].release_conformant());
}

#[test]
fn zero_and_coarse_timer_samples_remain_raw_and_inconclusive() {
    let (prepared, original) = fixture();
    for value in [0.0, 1.0, 10.0] {
        let mut trials = original.clone();
        for trial in &mut trials {
            trial.sample_ns.fill(value);
        }
        let report = compare(&prepared, &trials);
        assert_eq!(report.gate, Gate::Inconclusive);
        assert!(report.cases[0].resolution_limited);
        assert_eq!(report.cases[0].accepted_median_ns, Some(value));
        if value == 0.0 {
            assert_eq!(report.cases[0].median_ratio, None);
        }
        serde_json::to_string(&report).unwrap();
        assert!(trials.iter().all(|t| t.sample_ns == vec![value; 5]));
    }
    let mut trials = original.clone();
    trials[0].sample_ns[0] = 0.0;
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    trials[0].sample_ns[0] = -1.0;
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incomplete);
    let mut mixed = original.clone();
    for trial in &mut mixed {
        if trial.role == Role::Candidate {
            trial
                .sample_ns
                .fill(if trial.pair == 0 { 800.0 } else { 1200.0 });
        }
    }
    let report = compare(&prepared, &mixed);
    assert_eq!(report.gate, Gate::Inconclusive);
    assert!(!report.cases[0].resolution_limited);
    let (native, mut trials) = model::fixture();
    trials[0].sample_ns[0] = 0.0;
    assert_eq!(compare(&native, &trials).gate, Gate::Incomplete);
}

#[test]
fn preflight_mismatch_preserves_typed_output_without_claiming_timing() {
    let (prepared, trials) = fixture();
    let trial = &trials[0];
    let request = TrialRequest {
        role: trial.role,
        pair: trial.pair,
        reference_state: prepared.experiment.reference_state,
        executable: prepared.accepted.identity.clone(),
        case: prepared.experiment.cases[0].clone(),
        browser: Some(prepared.browser.as_ref().unwrap().trial(trial.role)),
        reference_output: Some(trial.reference.output.clone()),
    };
    let json = serde_json::to_vec(&request).unwrap();
    let decoded: TrialRequest = serde_json::from_slice(&json).unwrap();
    assert_eq!(decoded.reference_output, request.reference_output);
    let result = BrowserTransportResult {
        reference: None,
        prime_reference_output: None,
        role: trial.role,
        pair: trial.pair,
        case_name: trial.case_name.clone(),
        input: request.case.identity.input,
        settings: request.case.identity.settings,
        sample_ns: vec![],
        iterations_per_sample: 0,
        warmup_iterations: 0,
        warmup_elapsed_ns: 0,
        output: trial.output.output.clone(),
        unstable_output: None,
        observation: trial.browser.as_ref().unwrap().observation.clone(),
        timing_skipped: Some(TimingSkipped::ReferenceMismatch),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"timing_skipped\":\"reference-mismatch\""));
    let decoded: BrowserTransportResult = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.output, result.output);
    assert!(decoded.sample_ns.is_empty());
}

#[test]
fn incomplete_browser_identity_or_preparation_never_passes() {
    let (prepared, trials) = fixture();
    for mutation in 0..11 {
        let mut changed = trials.clone();
        let evidence = changed[0].browser.as_mut().unwrap();
        match mutation {
            0 => evidence.assets = content_digest(b"other assets"),
            1 => evidence.runtime = content_digest(b"other runtime"),
            2 => evidence.backend = BrowserBackend::Package,
            3 => evidence.preparation = BrowserPreparation::FreshInstance,
            4 => evidence.observation.browser_version = "other".into(),
            5 => evidence.observation.cross_origin_isolated = false,
            6 => evidence.observation.timer_resolution_ns = 0.0,
            7 => evidence.observation.node_version = "other".into(),
            8 => evidence.observation.playwright_version = "other".into(),
            9 => changed[0].browser = None,
            _ => changed[0].output.implementation.artifact = prepared.accepted.identity.clone(),
        }
        assert_eq!(
            compare(&prepared, &changed).gate,
            Gate::Incomplete,
            "mutation {mutation}"
        );
    }
    let mut missing = prepared.clone();
    missing.browser = None;
    assert_eq!(compare(&missing, &trials).gate, Gate::Incomplete);
    let (native, mut native_trials) = model::fixture();
    native_trials[0].browser = trials[0].browser.clone();
    assert_eq!(compare(&native, &native_trials).gate, Gate::Incomplete);
}

#[test]
fn unsupported_cache_anchor_init_and_fresh_throughput_are_rejected() {
    let (prepared, _) = fixture();
    for mutation in 0..5 {
        let mut case = prepared.experiment.cases[0].clone();
        match mutation {
            0 => case.measurement.application_cache = ApplicationCache::Cold,
            1 => {
                case.browser.as_mut().unwrap().operation = PublicOperation::ResizeNearest {
                    anchor: Anchor::Top,
                }
            }
            2 => {
                case.measurement.scope = CallScope::Initialization;
                case.browser.as_mut().unwrap().preparation =
                    BrowserPreparation::InitializationBytes;
            }
            3 => {
                case.measurement.mode = SampleMode::Throughput;
                case.browser.as_mut().unwrap().preparation = BrowserPreparation::FreshInstance;
            }
            _ => case.measurement.scope = CallScope::NativeKernel,
        }
        assert!(validate_case(&case).is_err(), "mutation {mutation}");
    }
    for preparation in [
        BrowserPreparation::InitializationBytes,
        BrowserPreparation::InitializationCompiled,
    ] {
        let mut case = prepared.experiment.cases[0].clone();
        case.measurement.scope = CallScope::Initialization;
        case.accepted_subject = case.candidate_subject.clone();
        let browser = case.browser.as_mut().unwrap();
        browser.accepted = BrowserBackend::Package;
        browser.preparation = preparation;
        validate_case(&case).unwrap();
    }
}

#[test]
fn content_identity_binds_assets_entrypoints_and_runtime_not_locations() {
    let (prepared, _) = fixture();
    let mut trial = prepared.browser.as_ref().unwrap().trial(Role::Accepted);
    validate_trial(&trial).unwrap();
    let original =
        artifact_identity(&prepared.accepted.identity, &trial.assets, &trial.runtime).unwrap();
    trial.assets.tree.root = "/relocated/assets".into();
    trial.runtime.browser.path = "/relocated/browser".into();
    trial.runtime.browser_assets.root = "/relocated".into();
    trial.runtime.node.path = "/relocated/node".into();
    trial.runtime.playwright.tree.root = "/relocated/playwright".into();
    assert_eq!(
        artifact_identity(&prepared.accepted.identity, &trial.assets, &trial.runtime).unwrap(),
        original
    );
    trial.runtime.launch_args.push("--different".into());
    assert_ne!(
        artifact_identity(&prepared.accepted.identity, &trial.assets, &trial.runtime).unwrap(),
        original
    );
    trial.assets.entries.package = "missing.js".into();
    assert!(validate_trial(&trial).is_err());
    for mutation in 0..4 {
        let mut trial = prepared.browser.as_ref().unwrap().trial(Role::Accepted);
        match mutation {
            0 => trial.assets.tree.files[0].digest = content_digest(b"different bytes"),
            1 => trial.assets.tree.files[0].path = "../escape".into(),
            2 => trial.assets.tree.files[0].mode = 0o644,
            _ => trial.assets.tree.files.reverse(),
        }
        assert!(validate_trial(&trial).is_err());
    }
}

#[test]
fn legacy_native_json_stays_readable_but_cannot_skip_browser_validation() {
    let (native, trials) = model::fixture();
    let json = serde_json::to_string(&native).unwrap();
    assert!(!json.contains("browser"));
    let restored: PreparedPair = serde_json::from_str(&json).unwrap();
    let restored_trials: Vec<TrialResult> =
        serde_json::from_str(&serde_json::to_string(&trials).unwrap()).unwrap();
    assert_eq!(compare(&restored, &restored_trials).gate, Gate::Pass);
    let (mut browser, _) = fixture();
    assert!(coordinator::run(&browser, std::path::Path::new("/unused"))
        .unwrap_err()
        .to_string()
        .contains("run_with_browser"));
    browser.browser = None;
    assert!(coordinator::run(&browser, std::path::Path::new("/unused")).is_err());
}

#[test]
#[cfg(target_os = "linux")]
fn browser_coordinator_checks_both_sides_of_each_child_and_preserves_failure_cleanup() {
    use ditherette_bench::lease::{Lease, QUIET_ENV};
    use std::{
        fs, io,
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };
    static VALIDATIONS: AtomicUsize = AtomicUsize::new(0);
    static FAIL_AT: AtomicUsize = AtomicUsize::new(usize::MAX);
    fn validate(trial: &BrowserTrial) -> io::Result<()> {
        validate_trial(trial)?;
        let index = VALIDATIONS.fetch_add(1, Ordering::SeqCst);
        if index == FAIL_AT.load(Ordering::SeqCst) {
            return Err(io::Error::other("controlled asset mutation"));
        }
        Ok(())
    }

    let directory = std::env::temp_dir().join(format!(
        "ditherette-browser-pair-fixture-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    std::env::set_var(QUIET_ENV, "1");
    std::env::set_var("DITHERETTE_PAIR_FIXTURE_DIRECTORY", &directory);
    let outcome = std::panic::catch_unwind(|| {
        let (template, _) = fixture();
        let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/paired-child.mjs");
        let prepared = coordinator::prepare_with_browser(
            template.experiment,
            (&script, &"a".repeat(40)),
            (&script, &"b".repeat(40)),
            &directory.join("prepared"),
            template.browser.unwrap(),
        )
        .unwrap();
        let report =
            coordinator::run_with_browser(&prepared, &directory.join("missing-proof"), validate)
                .unwrap();
        // The short fake child intentionally returns native output without browser evidence.
        assert_eq!(report.gate, Gate::Incomplete);
        assert_eq!(VALIDATIONS.load(Ordering::SeqCst), 8);
        assert!(!directory.join("active").exists());
        for (name, fail_at, expected_children) in [("before", 0, 0), ("after", 1, 1)] {
            VALIDATIONS.store(0, Ordering::SeqCst);
            FAIL_AT.store(fail_at, Ordering::SeqCst);
            let error = coordinator::run_with_browser(&prepared, &directory.join(name), validate)
                .unwrap_err();
            assert!(error.to_string().contains("controlled asset mutation"));
            let journal = fs::read_to_string(directory.join(name).join("events.jsonl")).unwrap();
            assert_eq!(
                journal
                    .lines()
                    .filter(|line| line.contains("\"state\":\"reaped\""))
                    .count(),
                expected_children
            );
            assert!(!directory.join("active").exists());
            drop(Lease::exclusive().unwrap());
        }
        VALIDATIONS.store(0, Ordering::SeqCst);
        FAIL_AT.store(usize::MAX, Ordering::SeqCst);
        std::env::set_var("DITHERETTE_PAIR_FIXTURE_FAILURE", "exit");
        assert!(coordinator::run_with_browser(
            &prepared,
            &directory.join("failed-child"),
            validate
        )
        .is_err());
        assert_eq!(VALIDATIONS.load(Ordering::SeqCst), 2);
        assert!(!directory.join("active").exists());
        drop(Lease::exclusive().unwrap());
    });
    std::env::remove_var(QUIET_ENV);
    std::env::remove_var("DITHERETTE_PAIR_FIXTURE_DIRECTORY");
    std::env::remove_var("DITHERETTE_PAIR_FIXTURE_FAILURE");
    fs::remove_dir_all(&directory).unwrap();
    outcome.unwrap();
}
