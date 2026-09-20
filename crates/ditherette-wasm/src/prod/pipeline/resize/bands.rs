//! Mutually exclusive scalar and worker scratch, owned by the existing preparation cache.

use std::mem::size_of;

use crate::{
    image::{ImageDimensions, ImageViewMut, Rgba8},
    prod::{
        contract::{error::ErrorCode, failure::Failure},
        pipeline::execution::RowBandPolicy,
        resize::common::allocation::CapacityBudget,
        tiling::{RowBand, RowBandBuffers},
    },
};

pub(in crate::prod::pipeline) enum ResizeScratch<T> {
    Scalar(Vec<T>),
    Bands(RowBandPolicy, RowBandBuffers<T>),
}

impl<T> From<Vec<T>> for ResizeScratch<T> {
    fn from(value: Vec<T>) -> Self {
        Self::Scalar(value)
    }
}

impl<T> ResizeScratch<T> {
    pub(super) fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Scalar(buffer) => (buffer.capacity() * size_of::<T>()) as u64,
            // Inline headers already belong to the enclosing PreparedResize record.
            Self::Bands(_, buffers) => {
                buffers.capacity_bytes() - size_of::<RowBandBuffers<T>>() as u64
            }
        }
    }

    pub(super) fn clear(&mut self) {
        *self = Self::Scalar(Vec::new());
    }

    pub(super) fn scalar(&mut self) -> &mut Vec<T> {
        let Self::Scalar(buffer) = self else {
            unreachable!("selected scalar scratch")
        };
        buffer
    }

    pub(super) fn execute(
        &mut self,
        output: &mut ImageViewMut<'_, Rgba8>,
        render: &(impl Fn(RowBand, ImageViewMut<'_, Rgba8>, &mut [T]) -> Result<(), Failure> + Sync),
        progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
    ) -> Option<Result<(), Failure>>
    where
        T: Send,
    {
        let Self::Bands(_, buffers) = self else {
            return None;
        };
        let dimensions = output.dimensions();
        Some((|| {
            progress(0, dimensions.height())?;
            buffers.execute(
                output.data_mut(),
                dimensions.width() as usize * 4,
                &|band, bytes, scratch| {
                    let local = ImageDimensions::new(dimensions.width(), band.height()).unwrap();
                    render(band, ImageViewMut::packed(bytes, local).unwrap(), scratch)?;
                    Ok(u64::from(band.height()))
                },
                &mut |completed| progress(completed as u32, dimensions.height()),
            )
        })())
    }
}

impl<T: Default + Clone> ResizeScratch<T> {
    pub(super) fn restore(
        &mut self,
        length: usize,
        budget: &mut CapacityBudget,
    ) -> Result<(), Failure> {
        match self {
            Self::Scalar(buffer) => super::restore_vector(buffer, length, budget),
            Self::Bands(_, _) => budget.check_additional(self.capacity_bytes()),
        }
    }

    /// Reserve complete worker capacity before output allocation. Budget rejection keeps scalar.
    pub(super) fn select(
        &mut self,
        output: ImageDimensions,
        policy: Option<RowBandPolicy>,
        scalar_length: usize,
        limit: u64,
        elements: &impl Fn(RowBand) -> Result<usize, Failure>,
    ) -> Result<(), Failure> {
        if let Some(policy) = policy {
            let required = RowBandBuffers::<T>::required_bytes(
                output,
                policy.height,
                policy.workers,
                policy.active_workers,
                elements,
            )?;
            let record = size_of::<RowBandBuffers<T>>() as u64;
            if required - record <= limit {
                if matches!(self, Self::Bands(current, _) if *current == policy)
                    && self.capacity_bytes() <= limit
                {
                    return Ok(());
                }
                self.clear();
                match RowBandBuffers::try_new(
                    output,
                    policy.height,
                    policy.workers,
                    policy.active_workers,
                    limit + record,
                    elements,
                ) {
                    Ok(buffers) => {
                        *self = Self::Bands(policy, buffers);
                        return Ok(());
                    }
                    Err(error) if error.code == ErrorCode::MemoryLimit => {}
                    Err(error) => return Err(error),
                }
            }
        }
        if matches!(self, Self::Bands(_, _)) {
            self.clear();
        }
        self.restore(scalar_length, &mut CapacityBudget::new(limit))
    }
}
