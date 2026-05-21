//! Area resize planning.
//!
//! The plan stores source-overlap spans for each output x/y coordinate. It keeps
//! coverage math out of the packed RGBA8 hot loop while preserving the exact
//! f64 weight calculation used by the scalar oracle-compatible kernel.

use crate::image::ImageDimensions;

use super::coverage::{clamp_i64, interval_overlap};

/// Reusable area-resize metadata for one source/output shape.
pub struct AreaResizePlan {
    pub(super) source_dimensions: ImageDimensions,
    pub(super) output_dimensions: ImageDimensions,
    pub(super) area: f64,
    pub(super) x_spans: Vec<Vec<AxisOverlap>>,
    pub(super) y_spans: Vec<Vec<AxisOverlap>>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AxisOverlap {
    pub(super) source_index: usize,
    pub(super) overlap: f64,
}

impl AreaResizePlan {
    /// Build reusable overlap spans for one exact area resize shape.
    pub fn new(source_dimensions: ImageDimensions, output_dimensions: ImageDimensions) -> Self {
        let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
        let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());
        let x_spans = axis_spans(
            source_dimensions.width(),
            output_dimensions.width(),
            x_scale,
        );
        let y_spans = axis_spans(
            source_dimensions.height(),
            output_dimensions.height(),
            y_scale,
        );

        // Keep raw overlaps plus area instead of pre-normalized weights. This
        // preserves the original f64 evaluation order: x_overlap * y_overlap / area.
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
}

fn axis_spans(source_len: u32, output_len: u32, scale: f64) -> Vec<Vec<AxisOverlap>> {
    (0..output_len)
        .map(|output_index| {
            let start = f64::from(output_index) * scale;
            let end = f64::from(output_index + 1) * scale;
            (start.floor() as i64..end.ceil() as i64)
                .filter_map(|source_index| {
                    let overlap = interval_overlap(
                        start,
                        end,
                        source_index as f64,
                        source_index as f64 + 1.0,
                    );
                    if overlap == 0.0 {
                        return None;
                    }

                    let source_index = clamp_i64(source_index, 0, i64::from(source_len) - 1);
                    Some(AxisOverlap {
                        source_index: source_index as usize,
                        overlap,
                    })
                })
                .collect()
        })
        .collect()
}
