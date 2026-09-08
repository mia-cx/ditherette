//! Generic work assignment for tiled execution.
//!
//! This module owns scheduler-neutral plumbing: turning output-space tiles/bands
//! plus a worker budget into per-worker work batches. Domain adapters still own
//! pixel semantics and the actual execution backend.

use super::{RowBand, RowBandPlan, WorkerBudget};
use crate::{
    image::ImageDimensions,
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
        },
        resize::common::allocation::CapacityBudget,
    },
};
use std::{mem::size_of, ops::Range};

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
    /// Complete assignment and band-vector capacity before any plan allocation.
    pub fn required_bytes(
        output: ImageDimensions,
        target_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
    ) -> Result<u64, Failure> {
        let bands = super::row_band::bands_for_output_height(output, target_height)
            .ok_or_else(|| Failure::new(ErrorCode::InvalidImage, ErrorPath::Output))?;
        let active = workers.active_workers(requested_workers, bands.len() as u32);
        Ok(bands.len() as u64 * size_of::<RowBand>() as u64
            + u64::from(active) * size_of::<RowBandWorkAssignment>() as u64)
    }

    /// Assigns contiguous row-band chunks to active workers.
    pub fn new(plan: &RowBandPlan, budget: WorkerBudget, requested_workers: u32) -> Option<Self> {
        let band_count = u32::try_from(plan.bands().len()).ok()?;
        let active_workers = budget.active_workers(requested_workers, band_count);
        if active_workers == 0 {
            return None;
        }

        let active_workers_usize = usize::try_from(active_workers).ok()?;
        let band_count = plan.bands().len();
        let assignments = assignment_ranges(band_count, active_workers_usize)
            .enumerate()
            .map(|(worker_index, range)| RowBandWorkAssignment {
                worker_index: worker_index as u32,
                bands: plan.bands()[range].to_vec(),
            })
            .collect::<Vec<_>>();

        Some(Self {
            requested_workers,
            active_workers,
            assignments,
        })
    }

    /// Build balanced assignments with every nested vector charged before pixel execution.
    /// No temporary complete band vector is allocated. Worker scratch remains the domain caller's responsibility.
    pub fn try_for_output_height(
        output: ImageDimensions,
        target_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
        capacity: &mut CapacityBudget,
    ) -> Result<Self, Failure> {
        let mut bands = super::row_band::bands_for_output_height(output, target_height)
            .ok_or_else(|| Failure::new(ErrorCode::InvalidImage, ErrorPath::Output))?;
        let band_count = bands.len();
        let active_workers = workers.active_workers(requested_workers, band_count as u32);
        capacity.check_additional(Self::required_bytes(
            output,
            target_height,
            workers,
            requested_workers,
        )?)?;
        let mut assignments = capacity.vector(active_workers as usize)?;
        for (worker_index, range) in
            assignment_ranges(band_count, active_workers as usize).enumerate()
        {
            let mut assigned = capacity.vector(range.len())?;
            assigned.extend(bands.by_ref().take(range.len()));
            assignments.push(RowBandWorkAssignment {
                worker_index: worker_index as u32,
                bands: assigned,
            });
        }
        Ok(Self {
            requested_workers,
            active_workers,
            assignments,
        })
    }

    /// Actual heap ownership, including assignment records and each nested band vector.
    pub fn capacity_bytes(&self) -> u64 {
        (self.assignments.capacity() * size_of::<RowBandWorkAssignment>()) as u64
            + self
                .assignments
                .iter()
                .map(|assignment| (assignment.bands.capacity() * size_of::<RowBand>()) as u64)
                .sum::<u64>()
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

pub(super) fn assignment_ranges(
    band_count: usize,
    workers: usize,
) -> impl Iterator<Item = Range<usize>> {
    let base = band_count / workers;
    let extra = band_count % workers;
    (0..workers).scan(0, move |next, worker| {
        let start = *next;
        *next += base + usize::from(worker < extra);
        Some(start..*next)
    })
}
