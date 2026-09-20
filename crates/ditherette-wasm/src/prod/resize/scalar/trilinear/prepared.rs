//! Full native preparation before source import. Every buffer stays owned until drop.
//! One storage-rounded chain supplies both adjacent levels without duplicate reductions.

use std::mem::size_of;

use super::{
    exact::{
        common::{alignment::ResizeAnchor, sample::ResizeSample},
        scalar::{
            area::resize_area_with_scratch_into, bilinear::resize_bilinear_with_scratch_into,
        },
    },
    half_rounded_up, minification_factor, MipLevel,
};
use crate::{
    image::{ImageDimensions, ImageFormat, ImageView, ImageViewMut},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
        },
        resize::common::allocation::CapacityBudget,
    },
};

/// Reserved chains, storage-rounded level outputs, and exact f64 channel scratch.
/// Caller-owned source/output buffers are excluded; the record and all owned capacities are included.
pub struct PreparedTrilinear<F: ImageFormat> {
    source: ImageDimensions,
    output: ImageDimensions,
    anchor: ResizeAnchor,
    blend: f64,
    levels: Vec<MipLevel<F>>,
    lower_count: usize,
    lower_output: Vec<F::Storage>,
    upper_output: Vec<F::Storage>,
    accumulated: Vec<f64>,
    capacity: u64,
}

impl<F: ImageFormat> PreparedTrilinear<F>
where
    F::Storage: ResizeSample,
{
    /// Peak native ownership when all preparations coexist. No source pixels are read.
    pub fn required_bytes(
        source: ImageDimensions,
        output: ImageDimensions,
    ) -> Result<u64, Failure> {
        let (lower, upper, _) = selection(source, output);
        let mut bytes = size_of::<Self>() as u64;
        add(&mut bytes, F::CHANNEL_COUNT, size_of::<f64>())?;
        let count = lower.max(upper);
        add(&mut bytes, count, size_of::<MipLevel<F>>())?;
        for dimensions in chain_dimensions(source, count) {
            add(
                &mut bytes,
                storage_len::<F>(dimensions)?,
                size_of::<F::Storage>(),
            )?;
        }
        if upper != 0 {
            for _ in 0..2 {
                add(
                    &mut bytes,
                    storage_len::<F>(output)?,
                    size_of::<F::Storage>(),
                )?;
            }
        }
        Ok(bytes)
    }

    /// Reserve every allocation before the caller imports source bytes.
    pub fn try_new(
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: ResizeAnchor,
        limit: u64,
    ) -> Result<Self, Failure> {
        let budget = CapacityBudget::new(limit);
        budget.check_additional(Self::required_bytes(source, output)?)?;
        let record = size_of::<Self>() as u64;
        let mut budget = CapacityBudget::new(limit - record);
        let (lower_count, upper_count, blend) = selection(source, output);
        let levels = reserve_chain(source, lower_count.max(upper_count), &mut budget)?;
        let output_len = if upper_count == 0 {
            0
        } else {
            storage_len::<F>(output)?
        };
        let mut lower_output = budget.vector(output_len)?;
        lower_output.resize(output_len, F::Storage::default());
        let mut upper_output = budget.vector(output_len)?;
        upper_output.resize(output_len, F::Storage::default());
        let mut accumulated = budget.vector(F::CHANNEL_COUNT)?;
        accumulated.resize(F::CHANNEL_COUNT, 0.0);
        Ok(Self {
            source,
            output,
            anchor,
            blend,
            levels,
            lower_count,
            lower_output,
            upper_output,
            accumulated,
            capacity: record + budget.used(),
        })
    }

    /// Prepared record plus actual owned vector capacities, excluding caller-owned images.
    pub fn capacity_bytes(&self) -> u64 {
        self.capacity
    }

    /// Execute without allocations. Every intermediate retains its frozen storage conversion.
    /// A mismatched view fails before writing output. Repeated execution refills the shared chain.
    pub fn execute(
        &mut self,
        source: ImageView<'_, F>,
        output: ImageViewMut<'_, F>,
    ) -> Result<(), Failure> {
        self.execute_with_progress(source, output, &mut |_, _| Ok(()))
    }

    /// Counts completed chain, sampling, and blending rows without replaying reductions.
    pub(crate) fn execute_with_progress(
        &mut self,
        source: ImageView<'_, F>,
        mut output: ImageViewMut<'_, F>,
        progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        if source.dimensions() != self.source {
            return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::Source));
        }
        if output.dimensions() != self.output {
            return Err(Failure::new(ErrorCode::InvalidSettings, ErrorPath::Output));
        }
        let chain_rows: u32 = self
            .levels
            .iter()
            .map(|level| level.dimensions.height())
            .sum();
        let total = chain_rows
            + self.output.height()
                * if self.lower_count < self.levels.len() {
                    3
                } else {
                    1
                };
        progress(0, total)?;
        if self.levels.is_empty() {
            resize_bilinear_with_scratch_into(source, output, self.anchor, &mut self.accumulated);
            return progress(total, total);
        }
        fill_chain(
            &source,
            &mut self.levels,
            &mut self.accumulated,
            &mut |completed| progress(completed, total),
        )?;
        let lower = self.levels[self.lower_count - 1].view();
        if self.lower_count == self.levels.len() {
            resize_bilinear_with_scratch_into(lower, output, self.anchor, &mut self.accumulated);
            return progress(total, total);
        }
        resize_bilinear_with_scratch_into(
            lower,
            ImageViewMut::<F>::packed(&mut self.lower_output, self.output).unwrap(),
            self.anchor,
            &mut self.accumulated,
        );
        progress(chain_rows + self.output.height(), total)?;
        resize_bilinear_with_scratch_into(
            self.levels.last().unwrap().view(),
            ImageViewMut::<F>::packed(&mut self.upper_output, self.output).unwrap(),
            self.anchor,
            &mut self.accumulated,
        );
        progress(chain_rows + self.output.height() * 2, total)?;
        let row_len = self.output.width_usize() * F::CHANNEL_COUNT;
        for y in 0..self.output.height() {
            let start = y as usize * row_len;
            for ((target, lower), upper) in output
                .row_mut(y)
                .unwrap()
                .iter_mut()
                .zip(&self.lower_output[start..start + row_len])
                .zip(&self.upper_output[start..start + row_len])
            {
                let blended = lower.to_f64() * (1.0 - self.blend) + upper.to_f64() * self.blend;
                *target = F::Storage::from_f64(blended);
            }
            progress(chain_rows + self.output.height() * 2 + y + 1, total)?;
        }
        Ok(())
    }
}

fn selection(source: ImageDimensions, output: ImageDimensions) -> (usize, usize, f64) {
    let minification = minification_factor(source, output);
    if minification <= 1.0 {
        return (0, 0, 0.0);
    }
    let lod = minification.log2();
    let lower_lod = lod.floor() as usize;
    let upper_lod = lod.ceil() as usize;
    let lower = chain_dimensions(source, lower_lod + 1).count();
    let lower_dimensions = chain_dimensions(source, lower).last().unwrap();
    let upper = if lower_lod == upper_lod
        || lower_dimensions.width() == 1 && lower_dimensions.height() == 1
    {
        0
    } else {
        chain_dimensions(source, upper_lod + 1).count()
    };
    (lower, upper, lod - lower_lod as f64)
}

fn chain_dimensions(
    source: ImageDimensions,
    count: usize,
) -> impl Iterator<Item = ImageDimensions> {
    std::iter::successors(Some(source), |current| {
        if current.width() == 1 && current.height() == 1 {
            return None;
        }
        Some(
            ImageDimensions::new(
                half_rounded_up(current.width()),
                half_rounded_up(current.height()),
            )
            .unwrap(),
        )
    })
    .take(count)
}

fn reserve_chain<F: ImageFormat>(
    source: ImageDimensions,
    count: usize,
    budget: &mut CapacityBudget,
) -> Result<Vec<MipLevel<F>>, Failure>
where
    F::Storage: ResizeSample,
{
    let mut chain = budget.vector(count)?;
    for dimensions in chain_dimensions(source, count) {
        let len = storage_len::<F>(dimensions)?;
        let mut data = budget.vector(len)?;
        data.resize(len, F::Storage::default());
        chain.push(MipLevel { dimensions, data });
    }
    Ok(chain)
}

fn fill_chain<F: ImageFormat>(
    source: &ImageView<'_, F>,
    chain: &mut [MipLevel<F>],
    accumulated: &mut [f64],
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure>
where
    F::Storage: ResizeSample,
{
    let row_len = source.dimensions().width_usize() * F::CHANNEL_COUNT;
    for y in 0..source.dimensions().height() {
        let start = y as usize * row_len;
        chain[0].data[start..start + row_len].copy_from_slice(source.row(y).unwrap());
        progress(y + 1)?;
    }
    let mut completed = source.dimensions().height();
    for index in 1..chain.len() {
        let (previous, next) = chain.split_at_mut(index);
        resize_area_with_scratch_into(
            previous[index - 1].view(),
            ImageViewMut::<F>::packed(&mut next[0].data, next[0].dimensions).unwrap(),
            accumulated,
        );
        completed += next[0].dimensions.height();
        progress(completed)?;
    }
    Ok(())
}

fn storage_len<F: ImageFormat>(dimensions: ImageDimensions) -> Result<usize, Failure> {
    dimensions.storage_len::<F>().map_err(|_| memory_limit())
}

fn add(total: &mut u64, count: usize, size: usize) -> Result<(), Failure> {
    *total = (count as u64)
        .checked_mul(size as u64)
        .and_then(|bytes| total.checked_add(bytes))
        .ok_or_else(memory_limit)?;
    Ok(())
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
