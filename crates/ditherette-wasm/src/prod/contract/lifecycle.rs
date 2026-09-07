//! Readable instance/control model for decisions 24, 36, and 37.
//!
//! This model owns no Wasm memory, callbacks, threads, or cache entries. Its transitions
//! specify when an implementation may call a callback or publish successful work.

use serde::{Deserialize, Serialize};

use super::{
    error::{DitheretteError, ErrorCode},
    request::{DEFAULT_MEMORY_LIMIT_BYTES, MAX_MEMORY_LIMIT_BYTES},
};

pub const PROGRESS_INTERVAL_MS: u64 = 50;
pub const MAX_CACHE_ENTRIES: usize = 128;
pub const MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;

/// Optional threading policy. Root import remains inert; disabled loads only scalar artifacts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Threads {
    #[default]
    Disabled,
    Preferred,
    Required,
}

/// Chosen initialization path; processing recipes never contain this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    Scalar,
    Threaded,
}

/// Reference initialization settings. Custom Wasm input stays at the JS loading boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitOptions {
    pub threads: Threads,
    pub memory_limit_bytes: u64,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            threads: Threads::Disabled,
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
        }
    }
}

impl InitOptions {
    /// Validates the explicit budget before allocating or selecting artifacts.
    pub fn validate(self) -> Result<Self, DitheretteError> {
        if self.memory_limit_bytes == 0 || self.memory_limit_bytes > MAX_MEMORY_LIMIT_BYTES {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                "memoryLimitBytes",
                "Memory limit must be between 1 byte and 2 GiB.",
            ));
        }
        Ok(self)
    }

    /// Cache capacity is subordinate to the instance's total memory limit.
    pub fn cache_limit_bytes(self) -> u64 {
        MAX_CACHE_BYTES.min(self.memory_limit_bytes / 4)
    }

    /// Counts planned private capacity and boundary copies, excluding caller/returned JS buffers and fixed overhead.
    pub fn preflight(self, required_capacity_bytes: u64) -> Result<(), DitheretteError> {
        self.validate()?;
        if required_capacity_bytes > self.memory_limit_bytes {
            return Err(DitheretteError::new(
                ErrorCode::MemoryLimit,
                "memoryLimitBytes",
                "Planned allocation capacity exceeds the instance memory limit.",
            ));
        }
        Ok(())
    }
}

/// Models capability/init fallback. Booleans describe completed attempts, not a command to load both builds.
pub fn initialize(
    options: InitOptions,
    threaded_capable: bool,
    threaded_initialized: bool,
    scalar_initialized: bool,
) -> Result<Execution, DitheretteError> {
    options.validate()?;
    if options.threads != Threads::Disabled {
        if threaded_capable && threaded_initialized {
            return Ok(Execution::Threaded);
        }
        if options.threads == Threads::Required {
            let code = if threaded_capable {
                ErrorCode::Initialization
            } else {
                ErrorCode::Capability
            };
            return Err(DitheretteError::new(
                code,
                "threads",
                "Required threaded initialization is unavailable.",
            ));
        }
    }
    if scalar_initialized {
        return Ok(Execution::Scalar);
    }
    Err(DitheretteError::new(
        ErrorCode::Initialization,
        "wasm",
        "Scalar initialization failed.",
    ))
}

/// Measurable public stages. Cache hits may omit any intermediate stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Prepare,
    Resize,
    Alpha,
    Color,
    Perturb,
    Quantize,
    DitherAndQuantize,
    Complete,
}

/// Optional counts describe completed work, never estimated remaining time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    pub stage: Stage,
    pub completed: Option<u64>,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum State {
    #[default]
    Ready,
    Running,
    Callback,
    Disposed,
}

/// Independent synchronous instance state. Successful finish is the sole cache-publication permission.
#[derive(Debug, Default)]
pub struct InstanceModel {
    state: State,
    progress_enabled: bool,
    last_progress: Option<(Stage, u64)>,
    output_ready: bool,
    completion_reported: bool,
}

impl InstanceModel {
    /// Starts one operation. Recursive processing fails without changing the active call.
    pub fn begin(&mut self, progress_enabled: bool) -> Result<(), DitheretteError> {
        self.require_idle()?;
        self.state = State::Running;
        self.progress_enabled = progress_enabled;
        self.last_progress = None;
        self.output_ready = false;
        self.completion_reported = false;
        Ok(())
    }

    /// Marks final result bytes and metadata ready, before completion or cache publication.
    pub fn output_ready(&mut self) -> Result<(), DitheretteError> {
        self.require_running()?;
        self.output_ready = true;
        Ok(())
    }

    /// Returns whether the implementation calls onProgress now. A true result enters callback state.
    pub fn report(&mut self, progress: Progress, now_ms: u64) -> Result<bool, DitheretteError> {
        self.require_running()?;
        if !self.progress_enabled {
            return Ok(false);
        }
        if self.completion_reported {
            return Err(control_error("Completion is the final progress event."));
        }
        if let (Some(completed), Some(total)) = (progress.completed, progress.total) {
            if completed > total {
                return Err(control_error("Completed work exceeds total work."));
            }
        }
        if progress.stage == Stage::Complete && !self.output_ready {
            return Err(control_error("Completion requires final output readiness."));
        }
        if let Some((stage, at)) = self.last_progress {
            if stage == progress.stage && now_ms.saturating_sub(at) < PROGRESS_INTERVAL_MS {
                return Ok(false);
            }
        }
        self.last_progress = Some((progress.stage, now_ms));
        self.completion_reported = progress.stage == Stage::Complete;
        self.state = State::Callback;
        Ok(true)
    }

    /// Resumes processing after the callback returns normally.
    pub fn callback_succeeded(&mut self) -> Result<(), DitheretteError> {
        if self.state != State::Callback {
            return Err(control_error("No callback is active."));
        }
        self.state = State::Running;
        Ok(())
    }

    /// Maps a thrown callback to a failed operation. The instance remains usable and publishes nothing.
    pub fn callback_failed(&mut self) -> DitheretteError {
        if self.state != State::Callback {
            return control_error("No callback is active.");
        }
        self.state = State::Ready;
        DitheretteError::new(
            ErrorCode::Callback,
            "onProgress",
            "Progress callback threw.",
        )
    }

    /// Finishes successfully and authorizes publication of this call's new cache entries.
    pub fn finish(&mut self) -> Result<(), DitheretteError> {
        self.require_running()?;
        if !self.output_ready || (self.progress_enabled && !self.completion_reported) {
            return Err(control_error(
                "Final output and enabled completion callback must finish before publication.",
            ));
        }
        self.state = State::Ready;
        Ok(())
    }

    /// Ends an expected processing failure without publishing new entries or a partial image.
    pub fn fail(&mut self) -> Result<(), DitheretteError> {
        self.require_running()?;
        self.state = State::Ready;
        Ok(())
    }

    /// Idempotently releases instance-owned allocations; recursive disposal is rejected.
    pub fn dispose(&mut self) -> Result<(), DitheretteError> {
        if self.state == State::Disposed {
            return Ok(());
        }
        self.require_idle()?;
        self.state = State::Disposed;
        Ok(())
    }

    fn require_idle(&self) -> Result<(), DitheretteError> {
        match self.state {
            State::Ready => Ok(()),
            State::Disposed => Err(DitheretteError::new(
                ErrorCode::Disposed,
                "instance",
                "Processor is disposed.",
            )),
            _ => Err(DitheretteError::new(
                ErrorCode::ReentrantCall,
                "instance",
                "Processing and disposal cannot reenter an active call.",
            )),
        }
    }

    fn require_running(&self) -> Result<(), DitheretteError> {
        if self.state == State::Running {
            return Ok(());
        }
        Err(control_error(
            "No processing operation is active outside a callback.",
        ))
    }
}

fn control_error(message: &str) -> DitheretteError {
    DitheretteError::new(ErrorCode::Runtime, "control", message)
}
