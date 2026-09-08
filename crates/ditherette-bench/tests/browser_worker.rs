use ditherette_bench::{
    browser_worker::*,
    paired::{browser::*, *},
    verification::content_digest,
};
use ditherette_bench_api::verification::*;

#[test]
fn process_transport_uses_recipe_output_dimensions_and_indexed_bounds() {
    use ditherette_bench::paired::process::ProcessSettings;
    use ditherette_wasm::{image::contracts::PaletteEntry, spec::contract::request as spec};
    let (mut request, mut result) = fixture();
    let output = Dimensions {
        width: 2,
        height: 3,
    };
    let operation = PublicOperation::Process {
        settings: ProcessSettings {
            palette: vec![PaletteEntry::Color { rgb: [1, 2, 3] }],
            recipe: spec::RecipeV1 {
                version: 1,
                output: spec::Output {
                    width: 2,
                    height: 3,
                    resize: spec::ResizePolicy::Nearest {
                        anchor: spec::Anchor::Center,
                    },
                },
                alpha: spec::AlphaPolicy::Premultiplied {},
                matching: spec::MatchPolicy::SrgbEuclidean,
                dither: spec::DitherPolicy::None {},
            },
        },
    };
    let case = &mut request.case;
    case.identity = operation.identity(case.source, &case.rgba, output).unwrap();
    case.reference_subject = operation.reference_subject().into();
    case.accepted_subject = operation.subject(BrowserBackend::Package).into();
    case.candidate_subject = case.accepted_subject.clone();
    case.browser.as_mut().unwrap().operation = operation;
    case.browser.as_mut().unwrap().accepted = BrowserBackend::Package;
    result.input = case.identity.input;
    result.settings = case.identity.settings;
    result.output.dimensions = output;
    result.output.pixels = Pixels::Indexed8 {
        indices: vec![0; 6],
        palette_rgba: vec![1, 2, 3, 255],
        transparent_index: None,
    };
    request.reference_output = Some(result.output.clone());
    result.reference = Some(OracleOutput {
        case: case.identity.clone(),
        output: result.output.clone(),
    });
    validate_response(&request, &result).unwrap();
    result.output.dimensions = request.case.source;
    assert!(validate_response(&request, &result).is_err());
}

#[test]
fn yliluoma_transport_accepts_indexed_output_and_rejects_rgba_output() {
    use ditherette_bench::paired::{quantize::*, yliluoma::YliluomaSettings};
    use ditherette_wasm::spec::contract::request::{BayerSize, Placement};

    let (mut request, mut result) = fixture();
    let operation = PublicOperation::Yliluoma {
        settings: YliluomaSettings {
            quantize: QuantizeSettings {
                palette: vec![PaletteEntry::Color { rgb: [1, 2, 3] }],
                alpha: AlphaPolicy::Premultiplied {},
                matching: MatchPolicy::SrgbEuclidean,
            },
            size: BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    };
    let case = &mut request.case;
    case.identity = operation
        .identity(case.source, &case.rgba, case.source)
        .unwrap();
    case.reference_subject = operation.reference_subject().into();
    case.accepted_subject = operation.subject(BrowserBackend::Package).into();
    case.candidate_subject = case.accepted_subject.clone();
    let browser = case.browser.as_mut().unwrap();
    browser.operation = operation;
    browser.accepted = BrowserBackend::Package;
    result.input = case.identity.input;
    result.settings = case.identity.settings;
    // A single opaque palette entry always emits its original index.
    result.output.pixels = Pixels::Indexed8 {
        indices: vec![0],
        palette_rgba: vec![1, 2, 3, 255],
        transparent_index: None,
    };
    request.reference_output = Some(result.output.clone());
    result.reference = Some(OracleOutput {
        case: case.identity.clone(),
        output: result.output.clone(),
    });
    validate_response(&request, &result).unwrap();
    result.output.pixels = Pixels::Rgba8 {
        data: vec![1, 2, 3, 255],
    };
    assert!(validate_response(&request, &result).is_err());
}

#[test]
fn browser_transport_requires_an_independently_identified_target_reference() {
    let (request, mut result) = fixture();
    result.reference = None;
    assert!(validate_response(&request, &result)
        .unwrap_err()
        .to_string()
        .contains("Wasm oracle"));
}

#[test]
fn transport_bound_retains_reference_and_both_large_unstable_outputs() {
    let (mut request, mut result) = fixture();
    let dimensions = Dimensions {
        width: 400,
        height: 400,
    };
    let output = VerificationOutput {
        dimensions,
        pixels: Pixels::Rgba8 {
            data: vec![255; 400 * 400 * 4],
        },
        warnings: vec![],
    };
    request.case.identity.output = dimensions;
    result.output = output.clone();
    result.unstable_output = Some(output.clone());
    result.reference.as_mut().unwrap().output = output;
    assert!(
        serde_json::to_vec(&result).unwrap().len() as u64 + 1 <= response_limit(&request).unwrap()
    );
}

#[test]
fn wasm_reference_identity_and_exact_mismatch_evidence_are_not_native_overrides() {
    let (request, mut result) = fixture();
    if let Pixels::Rgba8 { data } = &mut result.reference.as_mut().unwrap().output.pixels {
        data[0] = 94;
    }
    result.output = result.reference.as_ref().unwrap().output.clone();
    validate_response(&request, &result).unwrap();
    for mutate in [
        |v: &mut OracleOutput| v.case.input.0[0] ^= 1,
        |v: &mut OracleOutput| v.case.settings.0[0] ^= 1,
        |v: &mut OracleOutput| v.case.semantics.recipe.push('x'),
        |v: &mut OracleOutput| v.case.output.width += 1,
    ] {
        let mut changed = result.clone();
        mutate(changed.reference.as_mut().unwrap());
        assert!(validate_response(&request, &changed).is_err());
    }
    if let Pixels::Rgba8 { data } = &mut result.output.pixels {
        data[0] += 1;
    }
    result.timing_skipped = Some(TimingSkipped::ReferenceMismatch);
    result.sample_ns.clear();
    result.iterations_per_sample = 0;
    result.warmup_iterations = 0;
    result.warmup_elapsed_ns = 0;
    let directory =
        std::env::temp_dir().join(format!("wasm-reference-mismatch-{}", std::process::id()));
    preserve_reference_mismatch(&request, &result, &directory).unwrap();
    let retained: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("results.json")).unwrap()).unwrap();
    let outputs: ThreeWayOutputs = serde_json::from_value(retained["outputs"].clone()).unwrap();
    assert_eq!(
        outputs.reference.unwrap().output,
        result.reference.unwrap().output
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn field_and_diffusion_protocol_bind_native_identity_and_require_the_correct_public_output() {
    use ditherette_bench::paired::{fields::*, native::NativeOperation, quantize::*};
    use ditherette_wasm::bench_subjects::{self, BenchSubject};
    let perturb = PerturbPolicy {
        field: Field::Random { seed: 0x12345678 },
        space: WorkingSpace::Oklab,
        strength: 0.7,
        placement: Placement::Adaptive {
            radius: 1,
            threshold: 10.0,
            softness: 5.0,
        },
    };
    let separable = SeparableSettings {
        perturb,
        quantize: QuantizeSettings {
            palette: vec![
                PaletteEntry::Color { rgb: [1, 2, 3] },
                PaletteEntry::Transparent {},
            ],
            alpha: AlphaPolicy::Preserve { threshold: 0.5 },
            matching: MatchPolicy::OklchHueArc,
        },
    };
    let diffusion = ditherette_bench::paired::diffusion::DiffusionSettings {
        quantize: separable.quantize.clone(),
        kernel: ditherette_wasm::spec::contract::request::Diffusion::Atkinson,
        feedback: ditherette_wasm::spec::contract::request::DiffusionFeedback::Matching,
        strength: 0.75,
        serpentine: true,
        placement: perturb.placement,
    };
    for (operation, native) in [
        (
            PublicOperation::Diffusion {
                settings: diffusion.clone(),
            },
            NativeOperation::Diffusion {
                settings: diffusion,
            },
        ),
        (
            PublicOperation::Perturb { settings: perturb },
            NativeOperation::Perturb { settings: perturb },
        ),
        (
            PublicOperation::Separable {
                settings: separable.clone(),
            },
            NativeOperation::Separable {
                settings: separable,
            },
        ),
    ] {
        let (mut request, mut result) = fixture();
        let case = &mut request.case;
        case.identity = operation
            .identity(case.source, &case.rgba, case.source)
            .unwrap();
        assert_eq!(
            case.identity,
            native.identity(case.source, &case.rgba).unwrap()
        );
        assert!(operation
            .identity(
                case.source,
                &case.rgba,
                Dimensions {
                    width: 2,
                    height: 1
                }
            )
            .is_err());
        case.reference_subject = operation.reference_subject().into();
        case.accepted_subject = operation.subject(BrowserBackend::Package).into();
        case.candidate_subject = case.accepted_subject.clone();
        case.browser.as_mut().unwrap().accepted = BrowserBackend::Package;
        case.browser.as_mut().unwrap().operation = operation.clone();
        validate_case(case).unwrap();
        let registry = bench_subjects::bench_subjects();
        let BenchSubject::Conformance(reference) = registry
            .iter()
            .find(|subject| subject.descriptor().id.as_str() == operation.reference_subject())
            .unwrap()
        else {
            panic!("typed reference")
        };
        let reference_request = operation
            .processing_request(case.source, &case.rgba)
            .unwrap()
            .unwrap();
        result.output = (reference.run)(&reference_request).unwrap();
        result.input = case.identity.input;
        result.settings = case.identity.settings;
        request.reference_output = Some(result.output.clone());
        result.reference = Some(OracleOutput {
            case: case.identity.clone(),
            output: result.output.clone(),
        });
        validate_response(&request, &result).unwrap();
        let restored: TrialRequest =
            serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        assert_eq!(restored.case.identity, request.case.identity);
        assert_eq!(restored.case.browser, request.case.browser);
        result.output.pixels = match result.output.pixels {
            Pixels::Rgba8 { .. } => Pixels::Indexed8 {
                indices: vec![0],
                palette_rgba: vec![1, 2, 3, 255],
                transparent_index: None,
            },
            Pixels::Indexed8 { .. } => Pixels::Rgba8 {
                data: vec![1, 2, 3, 4],
            },
            _ => unreachable!(),
        };
        assert!(validate_response(&request, &result).is_err());
        request.case.browser.as_mut().unwrap().accepted = BrowserBackend::TypeScript;
        request.case.accepted_subject = operation.subject(BrowserBackend::TypeScript).into();
        assert!(validate_case(&request.case)
            .unwrap_err()
            .to_string()
            .contains("no faithful TypeScript"));
    }
}

#[test]
fn indexed_transport_preserves_metadata_and_rejects_malformed_indices() {
    use ditherette_bench::paired::quantize::*;
    let (mut request, mut result) = fixture();
    let operation = PublicOperation::Quantize {
        settings: QuantizeSettings {
            palette: vec![PaletteEntry::Color { rgb: [1, 2, 3] }],
            alpha: AlphaPolicy::Premultiplied {},
            matching: MatchPolicy::SrgbEuclidean,
        },
    };
    let case = &mut request.case;
    case.identity = operation
        .identity(case.source, &case.rgba, case.source)
        .unwrap();
    case.reference_subject = operation.reference_subject().into();
    case.accepted_subject = operation.subject(BrowserBackend::Package).into();
    case.candidate_subject = case.accepted_subject.clone();
    let browser = case.browser.as_mut().unwrap();
    browser.operation = operation;
    browser.accepted = BrowserBackend::Package;
    result.input = case.identity.input;
    result.settings = case.identity.settings;
    result.output.pixels = Pixels::Indexed8 {
        indices: vec![0],
        palette_rgba: vec![1, 2, 3, 255],
        transparent_index: None,
    };
    request.reference_output = Some(result.output.clone());
    result.reference = Some(OracleOutput {
        case: case.identity.clone(),
        output: result.output.clone(),
    });
    validate_response(&request, &result).unwrap();
    result.output.warnings.push(Warning {
        code: WarningCode::TransparentFallback,
        message: "fixture".into(),
    });
    // Metadata-only A/B/A is unstable even with an exact first output and no diagnostic opt-in.
    result.unstable_output = request.reference_output.clone();
    validate_response(&request, &result).unwrap();
    let directory = std::env::temp_dir().join(format!(
        "ditherette-browser-indexed-unstable-{}",
        std::process::id()
    ));
    assert!(reject_unstable_output(&request, &result, &directory).is_err());
    let retained: BrowserTransportResult =
        serde_json::from_slice(&std::fs::read(directory.join("transport.json")).unwrap()).unwrap();
    assert_eq!(retained.output, result.output);
    assert_eq!(retained.unstable_output, request.reference_output);
    std::fs::remove_dir_all(directory).unwrap();
    result.unstable_output = None;
    result.timing_skipped = Some(TimingSkipped::ReferenceMismatch);
    result.sample_ns.clear();
    result.iterations_per_sample = 0;
    result.warmup_iterations = 0;
    result.warmup_elapsed_ns = 0;
    validate_response(&request, &result).unwrap();
    if let Pixels::Indexed8 { indices, .. } = &mut result.output.pixels {
        indices[0] = 1;
    }
    assert!(validate_response(&request, &result).is_err());
}

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
        prime_reference_output: None,
        reference: Some(OracleOutput {
            case: identity.clone(),
            output: output.clone(),
        }),
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
        unstable_output: None,
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
            native: None,
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

#[test]
fn unstable_trial_retains_first_distinct_images_and_rejects_publication() {
    let (mut request, mut result) = fixture();
    let mut preflight = result.output.clone();
    if let Pixels::Rgba8 { data } = &mut preflight.pixels {
        data[0] = 99;
    }
    result.unstable_output = Some(preflight.clone());
    validate_response(&request, &result).unwrap(); // Instability also invalidates an ordinary exact trial.
    request.case.browser.as_mut().unwrap().measure_nonexact = true;
    validate_response(&request, &result).unwrap(); // The distinct successor can equal frozen bytes.
    let decoded: BrowserTransportResult =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert_eq!(decoded.unstable_output, Some(preflight));
    let directory = std::env::temp_dir().join(format!(
        "ditherette-browser-unstable-{}",
        std::process::id()
    ));
    let error = reject_unstable_output(&request, &decoded, &directory).unwrap_err();
    assert!(error
        .to_string()
        .contains("both actual outputs retained, trial rejected"));
    let raw: BrowserTransportResult =
        serde_json::from_slice(&std::fs::read(directory.join("transport.json")).unwrap()).unwrap();
    assert_eq!(raw.unstable_output, result.unstable_output);
    assert_eq!(raw.output, result.output);
    for (phase, first_byte) in [("first-output", 99), ("first-distinct-output", 1)] {
        let evidence: serde_json::Value = serde_json::from_slice(
            &std::fs::read(directory.join(phase).join("results.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            evidence["outputs"]["candidate"]["output"]["pixels"]["data"][0],
            first_byte
        );
        assert!(evidence["outputs"]["accepted"].is_null());
        assert!(directory.join(phase).join("candidate.png").is_file());
    }
    std::fs::remove_dir_all(&directory).unwrap();
    // The first result can equal the frozen reference and later become unstable without diagnostic opt-in.
    request.case.browser.as_mut().unwrap().measure_nonexact = false;
    std::mem::swap(result.unstable_output.as_mut().unwrap(), &mut result.output);
    validate_response(&request, &result).unwrap();
    assert!(reject_unstable_output(&request, &result, &directory).is_err());
    assert!(directory.join("first-output/results.json").is_file());
    assert!(directory
        .join("first-distinct-output/results.json")
        .is_file());
    std::fs::remove_dir_all(&directory).unwrap();
    result.unstable_output = Some(result.output.clone());
    assert!(validate_response(&request, &result).is_err());
}
