//! Spec contract for output-domain row-band tiling.
//!
//! Tiling partitions output writes only. These types describe that contract in a
//! small, testable form so future production schedulers and benchmark harnesses
//! can validate their plans without involving resize/color/dither algorithms.

use crate::image::ImageDimensions;

/// A half-open output row interval: `y_start..y_end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBand {
    y_start: u32,
    y_end: u32,
}

impl RowBand {
    pub fn new(y_start: u32, y_end: u32) -> Option<Self> {
        (y_start < y_end).then_some(Self { y_start, y_end })
    }

    pub const fn y_start(self) -> u32 {
        self.y_start
    }

    pub const fn y_end(self) -> u32 {
        self.y_end
    }

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
    /// Splits the complete output into consecutive bands, shortening the final band.
    pub fn for_output_height(dimensions: ImageDimensions, target_height: u32) -> Option<Self> {
        if target_height == 0 {
            return None;
        }
        let mut bands = Vec::new();
        let mut start = 0;
        while start < dimensions.height() {
            let end = start + target_height.min(dimensions.height() - start);
            bands.push(RowBand::new(start, end)?);
            start = end;
        }
        Self::new(dimensions, bands)
    }

    pub fn new(output_dimensions: ImageDimensions, bands: Vec<RowBand>) -> Option<Self> {
        covers_output_once(output_dimensions, &bands).then_some(Self {
            output_dimensions,
            bands,
        })
    }

    pub const fn output_dimensions(&self) -> ImageDimensions {
        self.output_dimensions
    }

    pub fn bands(&self) -> &[RowBand] {
        &self.bands
    }
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
