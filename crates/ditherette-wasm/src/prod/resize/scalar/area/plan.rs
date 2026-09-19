//! Area resize planning.
//!
//! The plan stores source-overlap spans for each output x/y coordinate. It keeps
//! coverage math out of the packed RGBA8 hot loop while leaving the packed
//! kernel free to choose the accumulator precision.

use crate::image::{rgba8, ImageDimensions};
use crate::prod::{contract::failure::Failure, resize::common::allocation::CapacityBudget};
use std::mem::size_of;

use super::coverage::interval_overlap;

/// Reusable area-resize metadata for one source/output shape.
pub struct AreaResizePlan {
    pub(super) source_dimensions: ImageDimensions,
    pub(super) output_dimensions: ImageDimensions,
    pub(super) area: f64,
    pub(super) x_spans: Vec<XAxisOverlapSpan>,
    pub(super) y_spans: Vec<Vec<AxisOverlap>>,
}

#[derive(Debug, Clone)]
pub(super) struct XAxisOverlapSpan {
    pub(super) first_byte_offset: usize,
    pub(super) last_exclusive_byte_offset: usize,
    pub(super) overlaps: Vec<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AxisOverlap {
    pub(super) source_index: usize,
    pub(super) overlap: f64,
}

impl AreaResizePlan {
    /// Complete metadata and one-call scratch reservation, excluding this record.
    pub fn required_bytes(
        source: ImageDimensions,
        output: ImageDimensions,
    ) -> Result<u64, Failure> {
        if fast_path(source, output) {
            return Ok(0);
        }
        let mut bytes = u64::from(output.width()) * size_of::<XAxisOverlapSpan>() as u64
            + u64::from(output.height()) * size_of::<Vec<AxisOverlap>>() as u64;
        for (source_len, output_len, element_size) in [
            (source.width(), output.width(), size_of::<f64>()),
            (source.height(), output.height(), size_of::<AxisOverlap>()),
        ] {
            let scale = f64::from(source_len) / f64::from(output_len);
            for coordinate in 0..output_len {
                bytes +=
                    overlaps(source_len, coordinate, scale).count() as u64 * element_size as u64;
            }
        }
        if source.width() != output.width() && source.height() != output.height() {
            bytes += u64::from(source.width()) * 4 * size_of::<f32>() as u64;
        }
        Ok(bytes)
    }

    /// Prepare the landed overlap layout without temporary vectors or infallible allocation.
    pub fn try_new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        budget: &mut CapacityBudget,
    ) -> Result<Self, Failure> {
        budget.check_additional(Self::required_bytes(source_dimensions, output_dimensions)?)?;
        let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
        let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());
        let mut plan = Self {
            source_dimensions,
            output_dimensions,
            area: x_scale * y_scale,
            x_spans: Vec::new(),
            y_spans: Vec::new(),
        };
        if fast_path(source_dimensions, output_dimensions) {
            return Ok(plan);
        }
        plan.x_spans = budget.vector(output_dimensions.width_usize())?;
        for coordinate in 0..output_dimensions.width() {
            let entries = overlaps(source_dimensions.width(), coordinate, x_scale);
            let first = entries
                .clone()
                .next()
                .expect("area x span should overlap at least one source pixel")
                .source_index;
            let last = entries
                .clone()
                .last()
                .expect("area x span should overlap at least one source pixel")
                .source_index;
            let mut values = budget.vector(entries.clone().count())?;
            values.extend(entries.map(|overlap| overlap.overlap));
            plan.x_spans.push(XAxisOverlapSpan {
                first_byte_offset: first * rgba8::RGBA8_CHANNELS,
                last_exclusive_byte_offset: (last + 1) * rgba8::RGBA8_CHANNELS,
                overlaps: values,
            });
        }
        plan.y_spans = budget.vector(output_dimensions.height_usize())?;
        for coordinate in 0..output_dimensions.height() {
            let entries = overlaps(source_dimensions.height(), coordinate, y_scale);
            let mut row = budget.vector(entries.clone().count())?;
            row.extend(entries);
            plan.y_spans.push(row);
        }
        Ok(plan)
    }

    pub fn capacity_bytes(&self) -> u64 {
        (self.x_spans.capacity() * size_of::<XAxisOverlapSpan>()
            + self.y_spans.capacity() * size_of::<Vec<AxisOverlap>>()) as u64
            + self
                .x_spans
                .iter()
                .map(|span| span.overlaps.capacity() as u64 * size_of::<f64>() as u64)
                .sum::<u64>()
            + self
                .y_spans
                .iter()
                .map(|span| span.capacity() as u64 * size_of::<AxisOverlap>() as u64)
                .sum::<u64>()
    }

    pub fn scratch_elements(&self) -> usize {
        if fast_path(self.source_dimensions, self.output_dimensions)
            || self.same_width()
            || self.same_height()
        {
            0
        } else {
            self.source_dimensions.width_usize() * rgba8::RGBA8_CHANNELS
        }
    }

    /// Build reusable overlap spans for one area resize shape.
    pub fn new(source_dimensions: ImageDimensions, output_dimensions: ImageDimensions) -> Self {
        let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
        let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());
        let x_spans = x_axis_spans(
            source_dimensions.width(),
            output_dimensions.width(),
            x_scale,
        );
        let y_spans = axis_spans(
            source_dimensions.height(),
            output_dimensions.height(),
            y_scale,
        );

        // Keep raw overlaps plus area instead of pre-normalized weights so the
        // packed kernel can choose the precision and evaluation order.
        Self {
            source_dimensions,
            output_dimensions,
            area: x_scale * y_scale,
            x_spans,
            y_spans,
        }
    }

    /// Return whether this plan was built for the given source/output shape.
    pub fn matches(
        &self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
    ) -> bool {
        self.source_dimensions == source_dimensions && self.output_dimensions == output_dimensions
    }

    pub(super) fn same_width(&self) -> bool {
        self.source_dimensions.width() == self.output_dimensions.width()
    }

    pub(super) fn same_height(&self) -> bool {
        self.source_dimensions.height() == self.output_dimensions.height()
    }
}

fn x_axis_spans(source_len: u32, output_len: u32, scale: f64) -> Vec<XAxisOverlapSpan> {
    axis_spans(source_len, output_len, scale)
        .into_iter()
        .map(|span| {
            let first_source_index = span
                .first()
                .expect("area x span should overlap at least one source pixel")
                .source_index;
            let last_source_index = span
                .last()
                .expect("area x span should overlap at least one source pixel")
                .source_index;

            XAxisOverlapSpan {
                first_byte_offset: first_source_index * rgba8::RGBA8_CHANNELS,
                last_exclusive_byte_offset: (last_source_index + 1) * rgba8::RGBA8_CHANNELS,
                overlaps: span.into_iter().map(|overlap| overlap.overlap).collect(),
            }
        })
        .collect()
}

fn axis_spans(source_len: u32, output_len: u32, scale: f64) -> Vec<Vec<AxisOverlap>> {
    (0..output_len)
        .map(|output_index| overlaps(source_len, output_index, scale).collect())
        .collect()
}

fn overlaps(
    source_len: u32,
    output_index: u32,
    scale: f64,
) -> impl Iterator<Item = AxisOverlap> + Clone {
    let start = f64::from(output_index) * scale;
    let end = f64::from(output_index + 1) * scale;
    (start.floor() as i64..end.ceil() as i64)
        .filter(move |&source_index| (0..i64::from(source_len)).contains(&source_index))
        .filter_map(move |source_index| {
            let overlap =
                interval_overlap(start, end, source_index as f64, source_index as f64 + 1.0);
            (overlap != 0.0).then_some(AxisOverlap {
                source_index: source_index as usize,
                overlap,
            })
        })
}

fn fast_path(source: ImageDimensions, output: ImageDimensions) -> bool {
    source == output
        || super::exact::exact_integer_downscale_steps_for_dimensions(source, output).is_some()
        || super::exact::exact_integer_upscale_steps_for_dimensions(source, output).is_some()
}
