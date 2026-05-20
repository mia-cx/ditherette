//! Shared helpers for normalized packed RGBA8 image storage.
//!
//! Production resize treats packed RGBA8 as the default hot-path layout. These
//! helpers keep that byte-layout knowledge near the image model so other filters
//! can share it without depending on nearest-neighbor internals.

use super::{ImageDimensions, RowStride};

pub const RGBA8_CHANNELS: usize = 4;

pub fn packed_row_byte_len(dimensions: ImageDimensions) -> usize {
    dimensions.width_usize() * RGBA8_CHANNELS
}

pub fn is_packed_stride(dimensions: ImageDimensions, stride: RowStride) -> bool {
    stride.elements() == packed_row_byte_len(dimensions)
}
