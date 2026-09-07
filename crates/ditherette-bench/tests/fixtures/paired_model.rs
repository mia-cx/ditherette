use ditherette_bench::{paired::*, verification::content_digest};
use ditherette_bench_api::verification::*;

pub fn fixture() -> (PreparedPair, Vec<TrialResult>) {
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
        native: None,
        browser: None,
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
        browser: None,
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
            identity: artifact.clone(),
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
            trials.push(TrialResult {
                browser: None,
                pair,
                role,
                case_name: "one-call".into(),
                build: BuildIdentity {
                    revision: "a".repeat(40),
                    dirty: false,
                    rustc: "rustc fixture".into(),
                    tool_version: "fixture".into(),
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
