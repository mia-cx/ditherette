use crate::error::ProcessingError;

/// Width and height for a tightly packed image buffer.
///
/// This type validates the shared dimension invariants once at the boundary so
/// processing functions can receive a clear, already-checked shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageDimensions {
    width: u32,
    height: u32,
}

impl ImageDimensions {
    /// Creates non-zero image dimensions.
    pub fn new(width: u32, height: u32) -> Result<Self, ProcessingError> {
        if width == 0 {
            return Err(ProcessingError::ZeroDimension { name: "width" });
        }

        if height == 0 {
            return Err(ProcessingError::ZeroDimension { name: "height" });
        }

        Ok(Self { width, height })
    }

    /// Returns the width in pixels.
    pub fn width(self) -> u32 {
        self.width
    }

    /// Returns the height in pixels.
    pub fn height(self) -> u32 {
        self.height
    }

    /// Returns the width as `usize` for slice indexing.
    pub fn width_usize(self) -> Result<usize, ProcessingError> {
        usize::try_from(self.width).map_err(|_| ProcessingError::SizeOverflow {
            context: "image width",
        })
    }

    /// Returns the height as `usize` for slice indexing.
    pub fn height_usize(self) -> Result<usize, ProcessingError> {
        usize::try_from(self.height).map_err(|_| ProcessingError::SizeOverflow {
            context: "image height",
        })
    }

    /// Returns the total pixel count.
    pub fn pixel_count(self) -> Result<usize, ProcessingError> {
        let pixel_count = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .ok_or(ProcessingError::SizeOverflow {
                context: "image pixel count",
            })?;

        usize::try_from(pixel_count).map_err(|_| ProcessingError::SizeOverflow {
            context: "image pixel count",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ImageDimensions;
    use crate::error::ProcessingError;

    #[test]
    fn rejects_zero_width() {
        assert_eq!(
            ImageDimensions::new(0, 1),
            Err(ProcessingError::ZeroDimension { name: "width" })
        );
    }

    #[test]
    fn rejects_zero_height() {
        assert_eq!(
            ImageDimensions::new(1, 0),
            Err(ProcessingError::ZeroDimension { name: "height" })
        );
    }
}
