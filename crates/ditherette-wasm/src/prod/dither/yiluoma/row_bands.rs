//! Optional exact row bands. Complete-call measurements own selection of this candidate.

use crate::{
    image::{ImageDimensions, ImageView, Rgba8},
    prod::{
        color::packed::Converter,
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::Placement,
        },
        dither::ordered::BayerSize,
        quantize::PreparedQuantizer,
        tiling::{bands_for_output_height, RowBandBuffers, WorkerBudget},
    },
};
use std::mem::size_of;

/// Owns scheduling metadata only. Workers borrow one palette and perform the unchanged literal search.
pub struct YliluomaBands {
    buffers: RowBandBuffers<()>,
}

impl YliluomaBands {
    /// Count all assignment storage plus one live temporary converter per active worker.
    /// The caller separately charges source, output, shared palette preparation, and boundary copies.
    pub fn required_bytes(
        dimensions: ImageDimensions,
        band_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
    ) -> Result<u64, Failure> {
        let buffers = RowBandBuffers::<()>::required_bytes(
            dimensions,
            band_height,
            workers,
            requested_workers,
            &|_| Ok(0),
        )?;
        Ok(buffers + converter_bytes(dimensions, band_height, workers, requested_workers)?)
    }

    /// Reserve only after the complete candidate requirement fits the remaining call budget.
    pub fn try_new(
        dimensions: ImageDimensions,
        band_height: u32,
        workers: WorkerBudget,
        requested_workers: u32,
        limit: u64,
    ) -> Result<Self, Failure> {
        let temporary = converter_bytes(dimensions, band_height, workers, requested_workers)?;
        let limit = limit
            .checked_sub(temporary)
            .ok_or_else(|| Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes))?;
        let buffers = RowBandBuffers::try_new(
            dimensions,
            band_height,
            workers,
            requested_workers,
            limit,
            &|_| Ok(0),
        )?;
        Ok(Self { buffers })
    }

    /// Actual owned metadata and peak concurrent temporary converters, including stack storage.
    pub fn capacity_bytes(&self) -> u64 {
        self.buffers.capacity_bytes()
            + u64::from(self.buffers.work().active_workers()) * size_of::<Converter>() as u64
    }

    /// Execute exact bands on the initialized pool, or sequentially in builds without threads.
    /// Progress runs on the caller after joined batches and reports completed rows.
    pub fn execute(
        &mut self,
        source: ImageView<'_, Rgba8>,
        prepared: &PreparedQuantizer,
        indices: &mut [u8],
        size: BayerSize,
        placement: Placement,
        progress: &mut impl FnMut(u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.buffers.execute(
            indices,
            source.dimensions().width_usize(),
            &|band, output, _| {
                super::request::dither_yiluoma_band_with_progress(
                    source,
                    prepared,
                    output,
                    size,
                    placement,
                    band,
                    |_| Ok(()),
                )?;
                Ok(u64::from(band.height()))
            },
            &mut |completed| progress(completed as u32),
        )
    }
}

fn converter_bytes(
    dimensions: ImageDimensions,
    band_height: u32,
    workers: WorkerBudget,
    requested_workers: u32,
) -> Result<u64, Failure> {
    let bands = bands_for_output_height(dimensions, band_height)
        .ok_or_else(|| Failure::new(ErrorCode::InvalidImage, ErrorPath::Output))?;
    let active = workers.active_workers(requested_workers, bands.len() as u32);
    // Matching conversion finishes before adaptive neighbor conversion begins.
    // Neighbors convert sequentially, so only one converter is live per worker.
    Ok(u64::from(active) * size_of::<Converter>() as u64)
}
