//! Request-specific scheduling with separate, development-only stage overrides.

use crate::prod::tiling::WorkerBudget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBandPolicy {
    pub height: u32,
    pub workers: WorkerBudget,
    pub active_workers: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionPolicy {
    pub resize: Option<RowBandPolicy>,
    pub indexed: Option<RowBandPolicy>,
    pub mixing: Option<RowBandPolicy>,
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutionStage {
    Resize,
    Indexed,
    Mixing,
}

impl ExecutionStage {
    pub(super) const fn mask(self) -> u8 {
        1 << self as u8
    }
}

impl ExecutionPolicy {
    pub(super) fn stage(self, stage: ExecutionStage) -> Option<RowBandPolicy> {
        match stage {
            ExecutionStage::Resize => self.resize,
            ExecutionStage::Indexed => self.indexed,
            ExecutionStage::Mixing => self.mixing,
        }
    }

    #[cfg(any(test, feature = "bench-subjects"))]
    pub(super) fn set_stage(&mut self, stage: ExecutionStage, policy: Option<RowBandPolicy>) {
        match stage {
            ExecutionStage::Resize => self.resize = policy,
            ExecutionStage::Indexed => self.indexed = policy,
            ExecutionStage::Mixing => self.mixing = policy,
        }
    }
}

/// Actual initialized capacity used to choose among measured worker configurations.
pub fn worker_budget() -> WorkerBudget {
    #[cfg(feature = "threads")]
    return WorkerBudget::new(rayon::current_num_threads() as u32);
    #[cfg(not(feature = "threads"))]
    WorkerBudget::new(1)
}

/// Apply actual pool capacity after resolving the request's measured policy or override.
pub(super) fn resolve(
    measured: Option<RowBandPolicy>,
    explicit: Option<Option<RowBandPolicy>>,
) -> Option<RowBandPolicy> {
    #[cfg(feature = "threads")]
    {
        let mut policy = explicit.unwrap_or(measured)?;
        let capacity = worker_budget();
        if explicit.is_none()
            && (policy.active_workers < 2
                || policy.active_workers > capacity.pool_size()
                || policy.active_workers > policy.workers.pool_size())
        {
            return None;
        }
        policy.workers = WorkerBudget::new(policy.workers.pool_size().min(capacity.pool_size()));
        policy.active_workers = policy
            .workers
            .active_workers(policy.active_workers, u32::MAX);
        Some(policy)
    }
    #[cfg(not(feature = "threads"))]
    {
        let _ = measured;
        // Native developer fixtures still exercise sequential row-band execution.
        // Normal scalar builds have no setter and never select automatic bands.
        explicit.flatten()
    }
}
