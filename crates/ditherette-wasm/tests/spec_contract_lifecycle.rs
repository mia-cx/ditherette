use ditherette_wasm::spec::contract::{
    error::ErrorCode,
    lifecycle::*,
    request::{DEFAULT_MEMORY_LIMIT_BYTES, MAX_MEMORY_LIMIT_BYTES},
};

fn progress(stage: Stage) -> Progress {
    Progress {
        stage,
        completed: None,
        total: None,
    }
}

#[test]
fn scalar_is_default_and_optional_threads_fall_back_only_when_allowed() {
    let defaults = InitOptions::default();
    assert_eq!(defaults.memory_limit_bytes, DEFAULT_MEMORY_LIMIT_BYTES);
    assert_eq!(
        initialize(defaults, false, false, true).unwrap(),
        Execution::Scalar
    );
    assert_eq!(
        initialize(defaults, true, true, true).unwrap(),
        Execution::Scalar
    );
    let preferred = InitOptions {
        threads: Threads::Preferred,
        ..defaults
    };
    assert_eq!(
        initialize(preferred, false, false, true).unwrap(),
        Execution::Scalar
    );
    assert_eq!(
        initialize(preferred, true, false, true).unwrap(),
        Execution::Scalar
    );
    assert_eq!(
        initialize(preferred, true, true, false).unwrap(),
        Execution::Threaded
    );
    let required = InitOptions {
        threads: Threads::Required,
        ..defaults
    };
    assert_eq!(
        initialize(required, false, false, true).unwrap_err().code,
        ErrorCode::Capability
    );
    assert_eq!(
        initialize(required, true, false, true).unwrap_err().code,
        ErrorCode::Initialization
    );
    assert_eq!(
        initialize(defaults, false, false, false).unwrap_err().code,
        ErrorCode::Initialization
    );
}

#[test]
fn memory_preflight_uses_capacity_and_keeps_cache_under_total_budget() {
    let options = InitOptions {
        memory_limit_bytes: 400,
        ..InitOptions::default()
    };
    assert_eq!(options.cache_limit_bytes(), 100);
    assert!(options.preflight(400).is_ok());
    assert_eq!(
        options.preflight(401).unwrap_err().code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(InitOptions::default().cache_limit_bytes(), 268_435_456);
    for limit in [0, MAX_MEMORY_LIMIT_BYTES + 1] {
        assert_eq!(
            InitOptions {
                memory_limit_bytes: limit,
                ..options
            }
            .validate()
            .unwrap_err()
            .path,
            "memoryLimitBytes"
        );
    }
}

#[test]
fn progress_throttles_within_stage_and_requires_ready_output_before_completion() {
    let mut instance = InstanceModel::default();
    instance.begin(true).unwrap();
    assert!(instance.report(progress(Stage::Resize), 0).unwrap());
    instance.callback_succeeded().unwrap();
    assert!(!instance.report(progress(Stage::Resize), 49).unwrap());
    assert!(instance.report(progress(Stage::Resize), 50).unwrap());
    instance.callback_succeeded().unwrap();
    assert!(instance.report(progress(Stage::Quantize), 51).unwrap());
    instance.callback_succeeded().unwrap();
    assert!(instance.report(progress(Stage::Complete), 52).is_err());
    instance.output_ready().unwrap();
    assert!(instance.finish().is_err());
    assert!(instance.report(progress(Stage::Complete), 52).unwrap());
    assert!(instance.finish().is_err());
    instance.callback_succeeded().unwrap();
    instance.finish().unwrap();
}

#[test]
fn callback_failure_never_reaches_successful_publication_and_preserves_other_instances() {
    let mut active = InstanceModel::default();
    let mut other = InstanceModel::default();
    active.begin(true).unwrap();
    active.output_ready().unwrap();
    active.report(progress(Stage::Complete), 0).unwrap();
    assert_eq!(
        active.begin(false).unwrap_err().code,
        ErrorCode::ReentrantCall
    );
    assert_eq!(active.dispose().unwrap_err().code, ErrorCode::ReentrantCall);
    other.begin(false).unwrap();
    other.output_ready().unwrap();
    other.finish().unwrap();
    assert_eq!(active.callback_failed().code, ErrorCode::Callback);
    assert!(active.finish().is_err());
    active.begin(false).unwrap();
    active.fail().unwrap();
    active.begin(false).unwrap();
    active.output_ready().unwrap();
    active.finish().unwrap();
}

#[test]
fn disposal_is_idempotent_and_processing_afterward_is_rejected() {
    let mut instance = InstanceModel::default();
    instance.dispose().unwrap();
    instance.dispose().unwrap();
    assert_eq!(instance.begin(false).unwrap_err().code, ErrorCode::Disposed);
}

#[test]
fn cache_hits_may_skip_stages_and_disabled_callbacks_need_no_completion_event() {
    let mut instance = InstanceModel::default();
    instance.begin(true).unwrap();
    instance.output_ready().unwrap();
    assert!(instance.report(progress(Stage::Complete), 0).unwrap());
    instance.callback_succeeded().unwrap();
    instance.finish().unwrap();
    instance.begin(false).unwrap();
    assert!(!instance.report(progress(Stage::Prepare), 0).unwrap());
    instance.output_ready().unwrap();
    instance.finish().unwrap();
}
