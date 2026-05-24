//! Generic work assignment for tiled execution.
//!
//! This module owns scheduler-neutral plumbing: turning output-space tiles/bands
//! plus a worker budget into per-worker work batches. Domain adapters still own
//! pixel semantics and the actual execution backend.

use super::{RowBand, RowBandPlan, WorkerBudget};

/// Row-band work assigned to one active worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowBandWorkAssignment {
    worker_index: u32,
    bands: Vec<RowBand>,
}

impl RowBandWorkAssignment {
    pub const fn worker_index(&self) -> u32 {
        self.worker_index
    }

    pub fn bands(&self) -> &[RowBand] {
        &self.bands
    }
}

/// A row-band work plan capped by pool budget and available bands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowBandWorkPlan {
    requested_workers: u32,
    active_workers: u32,
    assignments: Vec<RowBandWorkAssignment>,
}

impl RowBandWorkPlan {
    /// Assigns contiguous row-band chunks to active workers.
    pub fn new(plan: &RowBandPlan, budget: WorkerBudget, requested_workers: u32) -> Option<Self> {
        let band_count = u32::try_from(plan.bands().len()).ok()?;
        let active_workers = budget.active_workers(requested_workers, band_count);
        if active_workers == 0 {
            return None;
        }

        let active_workers_usize = usize::try_from(active_workers).ok()?;
        let chunk_size = plan.bands().len().div_ceil(active_workers_usize);
        let assignments = plan
            .bands()
            .chunks(chunk_size)
            .enumerate()
            .map(|(worker_index, bands)| RowBandWorkAssignment {
                worker_index: worker_index as u32,
                bands: bands.to_vec(),
            })
            .collect::<Vec<_>>();

        Some(Self {
            requested_workers,
            active_workers,
            assignments,
        })
    }

    pub const fn requested_workers(&self) -> u32 {
        self.requested_workers
    }

    pub const fn active_workers(&self) -> u32 {
        self.active_workers
    }

    pub fn assignments(&self) -> &[RowBandWorkAssignment] {
        &self.assignments
    }
}
