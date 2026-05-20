//! Row-stride representation and validation.
//!
//! Stride is stored in scalar storage elements, not pixels or bytes. For RGBA8,
//! a packed row stride is `width * 4`; for Oklab32 it is `width * 3`. This
//! matches hot-path flat arrays and avoids per-pixel destructuring.

use super::{
    dimensions::ImageDimensions,
    formats::ImageFormat,
    validate::{checked_row_len, ImageLayoutError},
};

/// Number of scalar storage elements between the start of consecutive rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowStride {
    elements: usize,
}

impl RowStride {
    pub fn new(elements: usize) -> Result<Self, ImageLayoutError> {
        if elements == 0 {
            return Err(ImageLayoutError::StrideTooSmall {
                stride: 0,
                row_len: 1,
            });
        }
        Ok(Self { elements })
    }

    pub fn packed<F: ImageFormat>(dimensions: ImageDimensions) -> Result<Self, ImageLayoutError> {
        Ok(Self {
            elements: checked_row_len::<F>(dimensions)?,
        })
    }

    pub const fn elements(self) -> usize {
        self.elements
    }

    pub fn validate_for<F: ImageFormat>(
        self,
        dimensions: ImageDimensions,
    ) -> Result<(), ImageLayoutError> {
        let row_len = checked_row_len::<F>(dimensions)?;
        if self.elements < row_len {
            return Err(ImageLayoutError::StrideTooSmall {
                stride: self.elements,
                row_len,
            });
        }
        Ok(())
    }
}
