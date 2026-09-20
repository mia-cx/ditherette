//! Fallible ownership for complete row work and one reusable scratch vector per active worker.

use std::mem::size_of;

use super::{execute_row_band_work, RowBand, RowBandWorkPlan, WorkerBudget, MAX_WORKER_BUDGET};
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

/// The caller reserves this record, assignment metadata, and every live worker scratch capacity together.
/// Source, kernel plans, and final output stay borrowed and must be charged by the enclosing call.
pub struct RowBandBuffers<T> {
    work: RowBandWorkPlan,
    scratch: Vec<Vec<T>>,
}

impl<T: Default + Clone> RowBandBuffers<T> {
    /// Complete capacity required before allocation. Each worker reuses its largest band's scratch.
    pub fn required_bytes(
        output: ImageDimensions,
        target_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
        elements: &impl Fn(RowBand) -> Result<usize, Failure>,
    ) -> Result<u64, Failure> {
        Self::layout(output, target_height, workers, requested_workers, elements)
            .map(|(bytes, _, _)| bytes)
    }

    /// Preflight all metadata, per-worker buffers, and support overlap before reserving anything.
    pub fn try_new(
        output: ImageDimensions,
        target_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
        limit: u64,
        elements: &impl Fn(RowBand) -> Result<usize, Failure>,
    ) -> Result<Self, Failure> {
        let (required, lengths, active) =
            Self::layout(output, target_height, workers, requested_workers, elements)?;
        CapacityBudget::new(limit).check_additional(required)?;
        let mut budget = CapacityBudget::new(limit - size_of::<Self>() as u64);
        let work = RowBandWorkPlan::try_for_output_height(
            output,
            target_height,
            workers,
            requested_workers,
            &mut budget,
        )?;
        let mut scratch = budget.vector(active)?;
        for &length in &lengths[..active] {
            let mut buffer = budget.vector(length)?;
            buffer.resize(length, T::default());
            scratch.push(buffer);
        }
        Ok(Self { work, scratch })
    }

    fn layout(
        output: ImageDimensions,
        target_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
        elements: &impl Fn(RowBand) -> Result<usize, Failure>,
    ) -> Result<(u64, [usize; MAX_WORKER_BUDGET as usize], usize), Failure> {
        let plan_bytes =
            RowBandWorkPlan::required_bytes(output, target_height, workers, requested_workers)?;
        let mut bands = super::row_band::bands_for_output_height(output, target_height)
            .expect("validated band height");
        let active = workers.active_workers(requested_workers, bands.len() as u32) as usize;
        let mut lengths = [0; MAX_WORKER_BUDGET as usize];
        for (index, range) in super::work::assignment_ranges(bands.len(), active).enumerate() {
            for band in bands.by_ref().take(range.len()) {
                lengths[index] = lengths[index].max(elements(band)?);
            }
        }
        let mut bytes =
            size_of::<Self>() as u64 + plan_bytes + (active * size_of::<Vec<T>>()) as u64;
        for length in &lengths[..active] {
            bytes = (*length as u64)
                .checked_mul(size_of::<T>() as u64)
                .and_then(|capacity| bytes.checked_add(capacity))
                .ok_or_else(|| Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes))?;
        }
        Ok((bytes, lengths, active))
    }
}

impl<T> RowBandBuffers<T> {
    /// Actual owned capacity, including this record and every nested vector header.
    pub fn capacity_bytes(&self) -> u64 {
        size_of::<Self>() as u64
            + self.work.capacity_bytes()
            + (self.scratch.capacity() * size_of::<Vec<T>>()) as u64
            + self
                .scratch
                .iter()
                .map(|buffer| buffer.capacity() as u64 * size_of::<T>() as u64)
                .sum::<u64>()
    }

    /// Borrow the immutable assignment metadata for domain work-count planning.
    pub fn work(&self) -> &RowBandWorkPlan {
        &self.work
    }

    /// Run bounded batches through the shared executor. Only the caller invokes progress.
    pub fn execute<P: Send, E: Send>(
        &mut self,
        output: &mut [P],
        row_elements: usize,
        render: &(impl Fn(RowBand, &mut [P], &mut [T]) -> Result<u64, E> + Sync),
        progress: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<(), E>
    where
        T: Send,
    {
        execute_row_band_work(
            &self.work,
            output,
            row_elements,
            &mut self.scratch,
            &|band, output, scratch| render(band, output, scratch),
            progress,
        )
    }
}
