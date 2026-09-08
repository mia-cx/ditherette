//! Per-instance pool ownership during initialization, fallback, and teardown.
//! Counts represent owned worker handles, not running threads. See `thread_pool.md`.

use super::{
    error::{DitheretteError, ErrorCode},
    lifecycle::{initialize, Execution, InitOptions, Threads},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    StartingThreaded,
    StartingScalar,
    Ready(Execution),
    InitializationFailed,
    Disposed,
    HostTerminated,
}

/// Ownership ledger for one selected artifact and its optional worker pool.
/// The package lifecycle separately rejects disposal during a synchronous call or callback.
#[derive(Debug)]
pub struct ThreadPoolModel {
    options: InitOptions,
    threaded_capable: bool,
    state: State,
    owned_workers: usize,
    released_workers: usize,
    owns_shared_memory: bool,
}

impl ThreadPoolModel {
    /// Validates options and selects the first attempt without starting real workers.
    /// A selected threaded attempt owns its shared-memory reference from this point.
    pub fn new(options: InitOptions, threaded_capable: bool) -> Result<Self, DitheretteError> {
        options.validate()?;
        if options.threads == Threads::Required && !threaded_capable {
            initialize(options, false, false, false)?;
        }
        let threaded = options.threads != Threads::Disabled && threaded_capable;
        Ok(Self {
            options,
            threaded_capable,
            state: if threaded {
                State::StartingThreaded
            } else {
                State::StartingScalar
            },
            owned_workers: 0,
            released_workers: 0,
            owns_shared_memory: threaded,
        })
    }

    /// Records ownership immediately when another worker starts, including partial initialization.
    pub fn worker_started(&mut self) -> Result<(), DitheretteError> {
        self.require(State::StartingThreaded)?;
        self.owned_workers += 1;
        Ok(())
    }

    /// Retains a successful pool, or cleans up before selecting fallback or returning an error.
    /// `None` means cleanup finished and the preferred scalar attempt may now start.
    pub fn threaded_finished(
        &mut self,
        succeeded: bool,
    ) -> Result<Option<Execution>, DitheretteError> {
        self.require(State::StartingThreaded)?;
        if succeeded {
            if self.owned_workers == 0 {
                return Err(control_error(
                    "A successful pool must own at least one worker.",
                ));
            }
            let execution = initialize(self.options, self.threaded_capable, true, false)?;
            self.state = State::Ready(execution);
            return Ok(Some(execution));
        }

        self.release_pool();
        if self.options.threads == Threads::Required {
            self.state = State::InitializationFailed;
            return initialize(self.options, self.threaded_capable, false, false).map(Some);
        }
        self.state = State::StartingScalar;
        Ok(None)
    }

    /// Resolves scalar initialization only after a scalar selection or completed threaded cleanup.
    pub fn scalar_finished(&mut self, succeeded: bool) -> Result<Execution, DitheretteError> {
        self.require(State::StartingScalar)?;
        let selected = initialize(self.options, self.threaded_capable, false, succeeded);
        self.state = match selected {
            Ok(execution) => State::Ready(execution),
            Err(_) => State::InitializationFailed,
        };
        selected
    }

    /// Execution becomes available only after the selected initialization finishes successfully.
    pub fn execution(&self) -> Option<Execution> {
        match self.state {
            State::Ready(execution) => Some(execution),
            _ => None,
        }
    }

    /// Number of worker handles still owned by this instance or its partial initialization.
    pub const fn owned_workers(&self) -> usize {
        self.owned_workers
    }

    /// Cumulative worker handles released by cleanup, disposal, or host termination.
    pub const fn released_workers(&self) -> usize {
        self.released_workers
    }

    /// Whether the threaded artifact's shared-memory reference remains owned.
    pub const fn owns_shared_memory(&self) -> bool {
        self.owns_shared_memory
    }

    /// Releases owned pool resources once. The caller first passes the package's reentry guard.
    pub fn dispose(&mut self) {
        if matches!(self.state, State::Disposed | State::HostTerminated) {
            return;
        }
        self.release_pool();
        self.state = State::Disposed;
    }

    /// Models host teardown of the processing worker and its associated pool, even during startup.
    /// This is a host event, not a sixth package method or synchronous cancellation API.
    pub fn host_terminated(&mut self) {
        if matches!(self.state, State::Disposed | State::HostTerminated) {
            return;
        }
        self.release_pool();
        self.state = State::HostTerminated;
    }

    fn release_pool(&mut self) {
        self.released_workers += self.owned_workers;
        self.owned_workers = 0;
        self.owns_shared_memory = false;
    }

    fn require(&self, expected: State) -> Result<(), DitheretteError> {
        if matches!(self.state, State::Disposed | State::HostTerminated) {
            return Err(DitheretteError::new(
                ErrorCode::Disposed,
                "instance",
                "Thread-pool owner is disposed.",
            ));
        }
        if self.state != expected {
            return Err(control_error("Initialization transition is out of order."));
        }
        Ok(())
    }
}

fn control_error(message: &str) -> DitheretteError {
    DitheretteError::new(ErrorCode::Runtime, "threads", message)
}
