//! Output row-band partitioning.

use crate::image::ImageDimensions;

/// A half-open output row interval: `y_start..y_end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBand {
    y_start: u32,
    y_end: u32,
}

impl RowBand {
    /// Creates a non-empty half-open row band.
    pub const fn new(y_start: u32, y_end: u32) -> Option<Self> {
        if y_start < y_end {
            Some(Self { y_start, y_end })
        } else {
            None
        }
    }

    /// First output row included in this band.
    pub const fn y_start(self) -> u32 {
        self.y_start
    }

    /// First output row after this band.
    pub const fn y_end(self) -> u32 {
        self.y_end
    }

    /// Number of output rows in this band.
    pub const fn height(self) -> u32 {
        self.y_end - self.y_start
    }
}

/// A complete non-overlapping row-band plan for an output image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowBandPlan {
    output_dimensions: ImageDimensions,
    bands: Vec<RowBand>,
}

impl RowBandPlan {
    /// Creates a plan from explicit bands, accepting only complete row coverage.
    pub fn new(output_dimensions: ImageDimensions, bands: Vec<RowBand>) -> Option<Self> {
        covers_output_once(output_dimensions, &bands).then_some(Self {
            output_dimensions,
            bands,
        })
    }

    /// Splits output rows into contiguous bands no taller than `target_height`.
    pub fn for_output_height(
        output_dimensions: ImageDimensions,
        target_height: u32,
    ) -> Option<Self> {
        Some(Self {
            output_dimensions,
            bands: bands_for_output_height(output_dimensions, target_height)?.collect(),
        })
    }

    /// Output dimensions covered exactly once by this plan.
    pub const fn output_dimensions(&self) -> ImageDimensions {
        self.output_dimensions
    }

    /// Contiguous row bands covering the output height.
    pub fn bands(&self) -> &[RowBand] {
        &self.bands
    }
}

/// Iterate complete output bands without allocating. The exact iterator length supports worker preflight.
pub fn bands_for_output_height(
    output: ImageDimensions,
    target_height: u32,
) -> Option<impl ExactSizeIterator<Item = RowBand>> {
    (target_height > 0).then(|| {
        (0..output.height())
            .step_by(target_height as usize)
            .map(move |y_start| RowBand {
                y_start,
                y_end: y_start.saturating_add(target_height).min(output.height()),
            })
    })
}

fn covers_output_once(output_dimensions: ImageDimensions, bands: &[RowBand]) -> bool {
    let mut expected_start = 0;

    for band in bands {
        if band.y_start != expected_start || band.y_end > output_dimensions.height() {
            return false;
        }
        expected_start = band.y_end;
    }

    expected_start == output_dimensions.height()
}
