//! Exact visible coverage: one bit per pixel, independent of dither and palette.

use crate::{image::ImageDimensions, prod::resize::common::allocation::CapacityBudget};

pub(crate) struct Coverage {
    dimensions: ImageDimensions,
    bits: Vec<u8>,
}

/// Workers borrow either retained bits or this call's completed selector cells.
/// Neither representation allocates or reruns analysis at a band boundary.
#[derive(Clone, Copy)]
pub(crate) enum CoverageRef<'a> {
    Packed(&'a Coverage),
    Cells(ImageDimensions, &'a [[f32; 3]]),
}

impl CoverageRef<'_> {
    #[inline(always)]
    pub(crate) fn covers(self, x: u32, y: u32) -> bool {
        match self {
            Self::Packed(mask) => mask.covers(x, y),
            Self::Cells(dimensions, cells) => super::covers(
                cells[y as usize * dimensions.width_usize() + x as usize][2],
                x,
                y,
            ),
        }
    }
}

impl Coverage {
    /// Takes coverage produced by the admitted GPU job for this exact image.
    /// The bridge never accepts a mask independently from its captured source.
    #[cfg(feature = "gpu")]
    pub(crate) fn from_gpu(dimensions: ImageDimensions, mut bits: Vec<u8>) -> Self {
        let bytes = Self::required_bytes(dimensions) as usize;
        assert!(bits.len() >= bytes && bits.len() < bytes + 4);
        bits.truncate(bytes);
        Self { dimensions, bits }
    }

    #[cfg(feature = "gpu")]
    pub(crate) fn into_bits(mut self) -> Vec<u8> {
        self.bits.resize(self.bits.len().div_ceil(4) * 4, 0);
        self.bits
    }

    pub(crate) fn required_bytes(dimensions: ImageDimensions) -> u64 {
        dimensions
            .pixel_count()
            .expect("validated dimensions")
            .div_ceil(8) as u64
    }

    /// Optional retention never makes an otherwise admitted render fail.
    pub(crate) fn try_new(
        dimensions: ImageDimensions,
        cells: &[[f32; 3]],
        limit: u64,
    ) -> Option<Self> {
        let pixels = dimensions.pixel_count().expect("validated dimensions");
        let width = dimensions.width_usize();
        let mut bits = CapacityBudget::new(limit)
            .vector::<u8>(pixels.div_ceil(8))
            .ok()?;
        bits.resize(pixels.div_ceil(8), 0);
        for (index, cell) in cells[..pixels].iter().enumerate() {
            if super::covers(cell[2], (index % width) as u32, (index / width) as u32) {
                bits[index / 8] |= 1 << (index % 8);
            }
        }
        Some(Self { dimensions, bits })
    }

    pub(crate) fn capacity_bytes(&self) -> u64 {
        self.bits.capacity() as u64
    }

    #[inline(always)]
    pub(crate) fn covers(&self, x: u32, y: u32) -> bool {
        let index = y as usize * self.dimensions.width_usize() + x as usize;
        self.bits[index / 8] & (1 << (index % 8)) != 0
    }
}
