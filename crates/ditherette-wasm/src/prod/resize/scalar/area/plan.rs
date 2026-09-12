//! Area resize planning.
//!
//! The plan stores source-overlap spans for each output x/y coordinate. It keeps
//! coverage math out of the packed RGBA8 hot loop while leaving the packed
//! kernel free to choose the accumulator precision.

use crate::image::{rgba8, ImageDimensions};

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
        .map(|output_index| {
            let start = f64::from(output_index) * scale;
            let end = f64::from(output_index + 1) * scale;
            (start.floor() as i64..end.ceil() as i64)
                .filter(|&source_index| (0..i64::from(source_len)).contains(&source_index))
                .filter_map(|source_index| {
                    let overlap = interval_overlap(
                        start,
                        end,
                        source_index as f64,
                        source_index as f64 + 1.0,
                    );
                    (overlap != 0.0).then_some(AxisOverlap {
                        source_index: source_index as usize,
                        overlap,
                    })
                })
                .collect()
        })
        .collect()
}
