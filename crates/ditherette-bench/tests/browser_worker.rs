use ditherette_bench::{
    browser_worker::*,
    paired::{browser::*, *},
    verification::content_digest,
};
use ditherette_bench_api::verification::*;

fn fixture() -> (TrialRequest, BrowserTransportResult) {
    let dimensions = Dimensions {
        width: 1,
        height: 1,
    };
    let output = VerificationOutput {
        dimensions,
        pixels: Pixels::Rgba8 {
            data: vec![1, 2, 3, 4],
        },
        warnings: vec![],
    };
    let browser = BrowserCase {
        operation: PublicOperation::ResizeNearest {
            anchor: Anchor::Center,
        },
        accepted: BrowserBackend::TypeScript,
        candidate: BrowserBackend::Package,
        preparation: BrowserPreparation::PrimedInstance,
        cache: CacheCapability::None,
        measure_nonexact: false,
    };
    let identity = browser
        .operation
        .identity(dimensions, &[1, 2, 3, 4], dimensions)
        .unwrap();
    let tree = AssetTree {
        root: "/fixture".into(),
        files: vec![],
        digest: content_digest(b"tree"),
    };
    let assets = AssetBundle {
        tree: tree.clone(),
        entries: AssetEntries {
            package: "package/index.js".into(),
            typescript: "typescript/index.js".into(),
            transport: "scripts/transport.mjs".into(),
            page: "scripts/page.mjs".into(),
            wasm: "package/core.wasm".into(),
        },
    };
    let runtime = BrowserRuntime {
        engine: BrowserEngine::Webkit,
        node: RuntimeBinary {
            path: "/node".into(),
            digest: content_digest(b"node"),
            version: "v24".into(),
        },
        browser: RuntimeBinary {
            path: "/fixture/webkit".into(),
            digest: content_digest(b"browser"),
            version: "1".into(),
        },
        browser_assets: tree.clone(),
        playwright: RuntimePackage {
            tree,
            entry: "index.mjs".into(),
            version: "2".into(),
        },
        launch_args: vec![],
        headless: true,
        cross_origin_isolated: true,
    };
    let result = BrowserTransportResult {
        role: Role::Candidate,
        pair: 0,
        case_name: "fixture".into(),
        input: identity.input,
        settings: identity.settings,
        sample_ns: vec![0.0, 1.0, 2.0, 3.0, 4.0],
        iterations_per_sample: 1,
        warmup_iterations: 1,
        warmup_elapsed_ns: 1,
        output: output.clone(),
        timing_skipped: None,
        observation: BrowserObservation {
            engine: runtime.engine,
            browser_version: runtime.browser.version.clone(),
            node_version: runtime.node.version.clone(),
            playwright_version: runtime.playwright.version.clone(),
            user_agent: "fixture-UA".into(),
            cross_origin_isolated: true,
            timer_resolution_ns: 1.0,
        },
    };
    let request = TrialRequest {
        role: result.role,
        pair: result.pair,
        reference_state: ReferenceState::Frozen,
        executable: ArtifactIdentity {
            revision: "a".repeat(40),
            content: content_digest(b"worker"),
        },
        reference_output: Some(output),
        case: PairCase {
            name: "fixture".into(),
            identity,
            source: dimensions,
            rgba: vec![1, 2, 3, 4],
            reference_subject: browser.operation.reference_subject().into(),
            accepted_subject: browser.operation.subject(browser.accepted).into(),
            candidate_subject: browser.operation.subject(browser.candidate).into(),
            browser: Some(browser),
            measurement: Measurement {
                mode: SampleMode::SingleCall,
                scope: CallScope::CompleteCall,
                application_cache: ApplicationCache::NotApplicable,
                samples: 5,
                measurement_ms: 10,
                warmup_ms: 1,
                target_sample_ms: 1,
            },
        },
        browser: Some(BrowserTrial { assets, runtime }),
    };
    (request, result)
}

#[test]
fn checked_response_preserves_zero_samples_and_rejects_mismatched_evidence() {
    let (request, result) = fixture();
    validate_response(&request, &result).unwrap();
    let mutations: Vec<fn(&mut BrowserTransportResult)> = vec![
        |v| v.role = Role::Accepted,
        |v| v.pair += 1,
        |v| v.case_name.push('x'),
        |v| v.input.0[0] ^= 1,
        |v| v.settings.0[0] ^= 1,
        |v| v.sample_ns[0] = f64::NAN,
        |v| v.sample_ns[0] = -1.0,
        |v| v.sample_ns.truncate(4),
        |v| v.iterations_per_sample = 2,
        |v| v.warmup_iterations = 0,
        |v| v.warmup_elapsed_ns = 0,
        |v| v.observation.browser_version.push('x'),
        |v| v.observation.node_version.push('x'),
        |v| v.observation.playwright_version.push('x'),
        |v| v.observation.cross_origin_isolated = false,
        |v| v.observation.timer_resolution_ns = 0.0,
        |v| v.output.dimensions.width = 2,
        |v| v.output.pixels = Pixels::Rgba8 { data: vec![] },
    ];
    for mutate in mutations {
        let mut changed = result.clone();
        mutate(&mut changed);
        assert!(validate_response(&request, &changed).is_err());
    }
    let mut changed = result;
    if let Pixels::Rgba8 { data } = &mut changed.output.pixels {
        data[0] = 99;
    }
    validate_response(&request, &changed).unwrap(); // Real byte differences go to S05, not a protocol error.
}

#[test]
fn preflight_mismatch_retains_actual_s05_outputs_without_timing() {
    let (request, mut result) = fixture();
    result.timing_skipped = Some(TimingSkipped::ReferenceMismatch);
    result.sample_ns.clear();
    result.iterations_per_sample = 0;
    result.warmup_iterations = 0;
    result.warmup_elapsed_ns = 0;
    assert!(validate_response(&request, &result).is_err());
    if let Pixels::Rgba8 { data } = &mut result.output.pixels {
        data[0] = 99;
    }
    validate_response(&request, &result).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "ditherette-browser-mismatch-{}",
        std::process::id()
    ));
    preserve_reference_mismatch(&request, &result, &directory).unwrap();
    let evidence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("results.json")).unwrap()).unwrap();
    assert_eq!(
        evidence["outputs"]["candidate"]["output"]["pixels"]["data"][0],
        99
    );
    assert!(evidence["outputs"]["accepted"].is_null());
    assert!(directory.join("candidate.png").is_file());
    std::fs::remove_dir_all(&directory).unwrap();
    result.warmup_iterations = 1;
    assert!(validate_response(&request, &result).is_err());
}

#[cfg(unix)]
#[test]
fn malformed_owned_node_is_terminated_and_reaped_without_a_browser() {
    use std::{
        fs,
        process::{Command, Stdio},
    };
    let directory = std::env::temp_dir().join(format!(
        "ditherette-browser-transport-{}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let marker = directory.join("closed");
    let pid_file = directory.join("pid");
    let lease = ditherette_bench::lease::Lease::exclusive().unwrap();
    let mut command = Command::new("node");
    command.args(["-e", "const fs=require('node:fs'); fs.writeFileSync(process.argv[2],String(process.pid)); process.on('SIGTERM',()=>{fs.writeFileSync(process.argv[1],'closed');process.exit(0)}); console.log('not-json');setInterval(()=>{},1000)"])
        .arg(&marker).arg(&pid_file).stdout(Stdio::piped()).stderr(Stdio::inherit());
    let mut raw = Vec::new();
    assert!(read_owned_transport(&lease, command, 4096, &mut raw).is_err());
    assert_eq!(raw, b"not-json\n");
    assert_eq!(fs::read_to_string(&marker).unwrap(), "closed");
    let pid = fs::read_to_string(pid_file).unwrap();
    assert!(!std::path::Path::new("/proc").join(pid).exists());
    drop(lease);
    fs::remove_dir_all(directory).unwrap();
}
