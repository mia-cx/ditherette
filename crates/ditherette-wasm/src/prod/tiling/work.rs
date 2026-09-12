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
    /// Zero-based index of the active worker assigned this batch.
    pub const fn worker_index(&self) -> u32 {
        self.worker_index
    }

    /// Contiguous row bands assigned to this worker.
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
        let band_count = plan.bands().len();
        let base_bands_per_worker = band_count / active_workers_usize;
        let extra_band_workers = band_count % active_workers_usize;
        let mut next_band = 0;
        let assignments = (0..active_workers_usize)
            .map(|worker_index| {
                let band_count =
                    base_bands_per_worker + usize::from(worker_index < extra_band_workers);
                let band_start = next_band;
                next_band += band_count;
                RowBandWorkAssignment {
                    worker_index: worker_index as u32,
                    bands: plan.bands()[band_start..next_band].to_vec(),
                }
            })
            .collect::<Vec<_>>();

        Some(Self {
            requested_workers,
            active_workers,
            assignments,
        })
    }

    /// Worker count requested by the caller before budget and band-count caps.
    pub const fn requested_workers(&self) -> u32 {
        self.requested_workers
    }

    /// Worker count actually used by this plan.
    pub const fn active_workers(&self) -> u32 {
        self.active_workers
    }

    /// Per-worker contiguous row-band assignments.
    pub fn assignments(&self) -> &[RowBandWorkAssignment] {
        &self.assignments
    }
}
