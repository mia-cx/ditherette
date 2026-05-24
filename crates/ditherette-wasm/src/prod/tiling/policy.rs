//! Shared tiling worker-budget policy.
//!
//! The pool size is process capacity. Individual tiled jobs choose an active
//! worker count from empirical policy and cap it by both the pool and the number
//! of available work items.

/// Maximum worker pool size used by production tiling policy.
pub const MAX_WORKER_BUDGET: u32 = 8;

/// Process-level worker capacity for tiled production work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerBudget {
    pool_size: u32,
}

impl WorkerBudget {
    /// Creates a non-zero worker budget, clamped to the production maximum.
    pub const fn new(pool_size: u32) -> Self {
        let pool_size = if pool_size == 0 {
            1
        } else if pool_size > MAX_WORKER_BUDGET {
            MAX_WORKER_BUDGET
        } else {
            pool_size
        };
        Self { pool_size }
    }

    /// Derives the default pool budget from logical CPU availability.
    ///
    /// The app keeps up to `clamp(cpus / 2, 1, 8)` workers available for tiled
    /// image work. Per-job policy then chooses how many of those workers to use.
    pub const fn from_available_parallelism(available_parallelism: u32) -> Self {
        Self::new(available_parallelism / 2)
    }

    /// Returns the maximum number of workers available to one tiled job.
    pub const fn pool_size(self) -> u32 {
        self.pool_size
    }

    /// Returns every active-worker candidate within this budget.
    pub fn worker_counts(self) -> impl Iterator<Item = u32> {
        1..=self.pool_size
    }

    /// Caps a requested active worker count by pool capacity and work items.
    pub const fn active_workers(self, requested_workers: u32, work_items: u32) -> u32 {
        if work_items == 0 {
            return 0;
        }
        let requested_workers = if requested_workers == 0 {
            1
        } else if requested_workers > self.pool_size {
            self.pool_size
        } else {
            requested_workers
        };
        if requested_workers > work_items {
            work_items
        } else {
            requested_workers
        }
    }

    /// Returns whether this requested count is a distinct measurable candidate.
    pub const fn can_use_workers(self, requested_workers: u32, work_items: u32) -> bool {
        requested_workers >= 1
            && requested_workers <= self.pool_size
            && requested_workers <= work_items
    }
}
