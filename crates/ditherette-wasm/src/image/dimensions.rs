//! Logical image dimensions and checked size math.
//!
//! This file keeps width/height validation and pixel/storage length calculations
//! in one small type so processing modules can trust dimensions after
//! construction. All multiplication is checked here because overflow is a layout
//! concern, not an algorithm concern.

use super::{
    formats::ImageFormat,
    validate::{checked_byte_len, checked_pixel_count, checked_storage_len, ImageLayoutError},
};

/// Logical image dimensions in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageDimensions {
    width: u32,
    height: u32,
}

impl ImageDimensions {
    /// Creates non-zero image dimensions.
    pub fn new(width: u32, height: u32) -> Result<Self, ImageLayoutError> {
        if width == 0 {
            return Err(ImageLayoutError::ZeroWidth);
        }
        if height == 0 {
            return Err(ImageLayoutError::ZeroHeight);
        }

        let dimensions = Self { width, height };
        checked_pixel_count(dimensions)?;
        Ok(dimensions)
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn width_usize(self) -> usize {
        self.width as usize
    }

    pub const fn height_usize(self) -> usize {
        self.height as usize
    }

    pub fn pixel_count(self) -> Result<usize, ImageLayoutError> {
        checked_pixel_count(self)
    }

    pub fn storage_len<F: ImageFormat>(self) -> Result<usize, ImageLayoutError> {
        checked_storage_len::<F>(self)
    }

    pub fn byte_len<F: ImageFormat>(self) -> Result<usize, ImageLayoutError> {
        checked_byte_len::<F>(self)
    }
}
