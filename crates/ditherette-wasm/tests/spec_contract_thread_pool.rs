use ditherette_wasm::spec::contract::{
    error::ErrorCode,
    lifecycle::{Execution, InitOptions, Threads},
    thread_pool::ThreadPoolModel,
};

fn options(threads: Threads) -> InitOptions {
    InitOptions {
        threads,
        ..InitOptions::default()
    }
}

#[test]
fn scalar_needs_neither_workers_nor_shared_memory() {
    for (threads, capable) in [
        (Threads::Disabled, false),
        (Threads::Disabled, true),
        (Threads::Preferred, false),
    ] {
        let mut pool = ThreadPoolModel::new(options(threads), capable).unwrap();
        assert_eq!(pool.worker_started().unwrap_err().code, ErrorCode::Runtime);
        assert_eq!(pool.owned_workers(), 0);
        assert!(!pool.owns_shared_memory());
        assert_eq!(pool.scalar_finished(true).unwrap(), Execution::Scalar);
        assert_eq!(pool.execution(), Some(Execution::Scalar));
        pool.dispose();
        pool.dispose();
        assert_eq!(pool.released_workers(), 0);
        assert_eq!(pool.execution(), None);
    }
}

#[test]
fn preferred_cleans_every_partial_worker_before_scalar_fallback() {
    for workers in [0, 1, 3] {
        let mut pool = ThreadPoolModel::new(options(Threads::Preferred), true).unwrap();
        for _ in 0..workers {
            pool.worker_started().unwrap();
        }
        assert_eq!(pool.owned_workers(), workers);
        assert!(pool.owns_shared_memory());
        assert_eq!(
            pool.scalar_finished(true).unwrap_err().code,
            ErrorCode::Runtime
        );
        assert_eq!(pool.owned_workers(), workers);
        assert_eq!(pool.threaded_finished(false).unwrap(), None);
        assert_eq!(pool.owned_workers(), 0);
        assert_eq!(pool.released_workers(), workers);
        assert!(!pool.owns_shared_memory());
        assert_eq!(pool.scalar_finished(true).unwrap(), Execution::Scalar);
        pool.dispose();
        assert_eq!(pool.released_workers(), workers);
    }
}

#[test]
fn required_reports_capability_or_initialization_failure_without_fallback() {
    assert_eq!(
        ThreadPoolModel::new(options(Threads::Required), false)
            .unwrap_err()
            .code,
        ErrorCode::Capability
    );
    let mut pool = ThreadPoolModel::new(options(Threads::Required), true).unwrap();
    pool.worker_started().unwrap();
    pool.worker_started().unwrap();
    let error = pool.threaded_finished(false).unwrap_err();
    assert_eq!(error.code, ErrorCode::Initialization);
    assert_eq!(error.path, "threads");
    assert_eq!(pool.owned_workers(), 0);
    assert_eq!(pool.released_workers(), 2);
    assert!(!pool.owns_shared_memory());
    assert_eq!(
        pool.scalar_finished(true).unwrap_err().code,
        ErrorCode::Runtime
    );
    pool.dispose();
    assert_eq!(pool.released_workers(), 2);
}

#[test]
fn successful_pools_belong_to_one_instance_until_idempotent_disposal() {
    let mut first = ThreadPoolModel::new(options(Threads::Preferred), true).unwrap();
    let mut second = ThreadPoolModel::new(options(Threads::Required), true).unwrap();
    assert_eq!(
        first.threaded_finished(true).unwrap_err().code,
        ErrorCode::Runtime
    );
    first.worker_started().unwrap();
    second.worker_started().unwrap();
    second.worker_started().unwrap();
    assert_eq!(
        first.threaded_finished(true).unwrap(),
        Some(Execution::Threaded)
    );
    assert_eq!(
        second.threaded_finished(true).unwrap(),
        Some(Execution::Threaded)
    );
    first.dispose();
    first.dispose();
    assert_eq!(first.released_workers(), 1);
    assert_eq!(first.owned_workers(), 0);
    assert!(!first.owns_shared_memory());
    assert_eq!(
        first.worker_started().unwrap_err().code,
        ErrorCode::Disposed
    );
    assert_eq!(second.execution(), Some(Execution::Threaded));
    assert_eq!(second.owned_workers(), 2);
    assert!(second.owns_shared_memory());
}

#[test]
fn host_termination_releases_partial_or_ready_pools_and_cannot_resume_them() {
    for ready in [false, true] {
        let mut pool = ThreadPoolModel::new(options(Threads::Preferred), true).unwrap();
        pool.worker_started().unwrap();
        if ready {
            pool.threaded_finished(true).unwrap();
        }
        pool.host_terminated();
        pool.host_terminated();
        pool.dispose();
        assert_eq!(pool.execution(), None);
        assert_eq!(pool.owned_workers(), 0);
        assert_eq!(pool.released_workers(), 1);
        assert!(!pool.owns_shared_memory());
        assert_eq!(
            pool.scalar_finished(true).unwrap_err().code,
            ErrorCode::Disposed
        );
        assert_eq!(
            pool.threaded_finished(true).unwrap_err().code,
            ErrorCode::Disposed
        );
    }
}

#[test]
fn scalar_failure_and_invalid_options_keep_existing_structured_errors() {
    let invalid = InitOptions {
        memory_limit_bytes: 0,
        ..options(Threads::Required)
    };
    assert_eq!(
        ThreadPoolModel::new(invalid, false).unwrap_err().code,
        ErrorCode::InvalidSettings
    );
    let mut pool = ThreadPoolModel::new(options(Threads::Preferred), true).unwrap();
    pool.worker_started().unwrap();
    pool.threaded_finished(false).unwrap();
    let error = pool.scalar_finished(false).unwrap_err();
    assert_eq!(error.code, ErrorCode::Initialization);
    assert_eq!(error.path, "wasm");
    assert_eq!(pool.owned_workers(), 0);
    assert_eq!(pool.released_workers(), 1);
    assert!(!pool.owns_shared_memory());
    assert_eq!(pool.execution(), None);
}
