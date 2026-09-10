//! Optional exact RGB work, allocated only after mandatory call ownership fits.

use std::mem::size_of;

use crate::{
    image::{ImageDimensions, ImageView, Rgba8},
    prod::{
        contract::failure::Failure,
        pipeline::execution::RowBandPolicy,
        quantize::{cache::recommended_entries, PreparedQuantizer},
        resize::common::allocation::CapacityBudget,
        tiling::RowBandBuffers,
    },
};

pub(super) enum Work {
    Scalar(Vec<u64>),
    Bands(RowBandBuffers<u64>),
}

impl Work {
    /// Budget or reservation misses leave the caller's mandatory direct scan usable.
    pub(super) fn try_new(
        dimensions: ImageDimensions,
        bands: Option<RowBandPolicy>,
        available: u64,
    ) -> Option<Self> {
        let heap = available.checked_sub(size_of::<Self>() as u64)?;
        let work = if let Some(policy) = bands {
            let metadata = RowBandBuffers::<u64>::required_bytes(
                dimensions,
                policy.height,
                policy.workers,
                policy.active_workers,
                &|_| Ok(0),
            )
            .ok()?;
            let extra = metadata - size_of::<RowBandBuffers<u64>>() as u64;
            let workers = policy.workers.active_workers(
                policy.active_workers,
                dimensions.height().div_ceil(policy.height),
            );
            let entries = recommended_entries(
                dimensions.width_usize() * dimensions.height().min(policy.height) as usize,
                heap.checked_sub(extra)? / u64::from(workers),
            );
            if entries == 0 {
                return None;
            }
            Self::Bands(
                RowBandBuffers::try_new(
                    dimensions,
                    policy.height,
                    policy.workers,
                    policy.active_workers,
                    heap + size_of::<RowBandBuffers<u64>>() as u64,
                    &|_| Ok(entries),
                )
                .ok()?,
            )
        } else {
            let entries = recommended_entries(
                dimensions.pixel_count().expect("validated dimensions"),
                heap,
            );
            if entries == 0 {
                return None;
            }
            let mut table = CapacityBudget::new(heap).vector(entries).ok()?;
            table.resize(entries, 0);
            Self::Scalar(table)
        };
        Some(work)
    }

    pub(super) fn capacity_bytes(&self) -> u64 {
        size_of::<Self>() as u64
            + match self {
                Self::Scalar(table) => (table.capacity() * size_of::<u64>()) as u64,
                Self::Bands(bands) => {
                    bands.capacity_bytes() - size_of::<RowBandBuffers<u64>>() as u64
                }
            }
    }

    pub(super) fn execute(
        &mut self,
        prepared: &PreparedQuantizer,
        source: ImageView<'_, Rgba8>,
        output: &mut [u8],
        progress: &mut impl FnMut(u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        match self {
            Self::Scalar(entries) => {
                prepared.quantize_cached_with_progress(source, output, entries, progress)
            }
            Self::Bands(bands) => {
                prepared.quantize_cached_bands_into(source, output, bands, &mut |rows| {
                    progress(rows as u32)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prod::tiling::WorkerBudget;

    #[test]
    fn optional_work_accounts_its_record_and_every_worker_table() {
        let dimensions = ImageDimensions::new(64, 64).unwrap();
        let scalar_minimum = size_of::<Work>() as u64 + 1024 * 8;
        assert!(Work::try_new(dimensions, None, scalar_minimum - 1).is_none());
        assert_eq!(
            Work::try_new(dimensions, None, scalar_minimum)
                .unwrap()
                .capacity_bytes(),
            scalar_minimum
        );
        let policy = RowBandPolicy {
            height: 32,
            workers: WorkerBudget::new(4),
            active_workers: 4,
        };
        let metadata =
            RowBandBuffers::<u64>::required_bytes(dimensions, 32, policy.workers, 4, &|_| Ok(0))
                .unwrap();
        let bands_minimum = size_of::<Work>() as u64 + metadata
            - size_of::<RowBandBuffers<u64>>() as u64
            + 2 * 1024 * 8;
        assert!(Work::try_new(dimensions, Some(policy), bands_minimum - 1).is_none());
        assert_eq!(
            Work::try_new(dimensions, Some(policy), bands_minimum)
                .unwrap()
                .capacity_bytes(),
            bands_minimum
        );
        for limit in [0, scalar_minimum, bands_minimum, 1 << 20] {
            for bands in [None, Some(policy)] {
                if let Some(work) = Work::try_new(dimensions, bands, limit) {
                    assert!(work.capacity_bytes() <= limit);
                }
            }
        }
    }
}
