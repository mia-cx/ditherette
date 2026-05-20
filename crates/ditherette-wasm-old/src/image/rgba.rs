use crate::{error::ProcessingError, image::ImageDimensions};

/// Number of bytes per canonical RGBA pixel.
pub const RGBA_CHANNEL_COUNT: usize = 4;

const MAX_WASM32_BYTE_LENGTH: usize = u32::MAX as usize;

/// Returns the byte length required for a tightly packed RGBA buffer.
///
/// The current Wasm target is `wasm32-unknown-unknown`, so this rejects buffers
/// whose calculated byte length cannot fit in 32-bit linear-memory addressing.
pub fn checked_rgba_byte_len(dimensions: ImageDimensions) -> Result<usize, ProcessingError> {
    let byte_len = dimensions
        .pixel_count()?
        .checked_mul(RGBA_CHANNEL_COUNT)
        .ok_or(ProcessingError::SizeOverflow {
            context: "RGBA byte length",
        })?;

    if byte_len > MAX_WASM32_BYTE_LENGTH {
        return Err(ProcessingError::SizeOverflow {
            context: "RGBA byte length",
        });
    }

    Ok(byte_len)
}

/// Verifies that an RGBA byte slice exactly matches its declared dimensions.
pub fn validate_rgba_buffer(
    source_rgba: &[u8],
    dimensions: ImageDimensions,
) -> Result<(), ProcessingError> {
    let expected = checked_rgba_byte_len(dimensions)?;
    let actual = source_rgba.len();

    if actual != expected {
        return Err(ProcessingError::InvalidBufferLength { expected, actual });
    }

    Ok(())
}

/// Returns the byte offset of a pixel in a tightly packed RGBA buffer.
///
/// Callers are responsible for passing coordinates inside the image bounds.
pub fn pixel_byte_offset(width: usize, x: usize, y: usize) -> usize {
    (y * width + x) * RGBA_CHANNEL_COUNT
}

#[cfg(test)]
mod tests {
    use super::{checked_rgba_byte_len, validate_rgba_buffer};
    use crate::{error::ProcessingError, image::ImageDimensions};

    #[test]
    fn calculates_rgba_byte_length() {
        let dimensions = ImageDimensions::new(3, 2).unwrap();

        assert_eq!(checked_rgba_byte_len(dimensions), Ok(24));
    }

    #[test]
    fn rejects_overflowing_rgba_byte_length() {
        let dimensions = ImageDimensions::new(u32::MAX, u32::MAX).unwrap();

        assert!(matches!(
            checked_rgba_byte_len(dimensions),
            Err(ProcessingError::SizeOverflow { .. })
        ));
    }

    #[test]
    fn validates_matching_buffer_length() {
        let dimensions = ImageDimensions::new(1, 2).unwrap();
        let source_rgba = [0; 8];

        assert_eq!(validate_rgba_buffer(&source_rgba, dimensions), Ok(()));
    }

    #[test]
    fn rejects_mismatched_buffer_length() {
        let dimensions = ImageDimensions::new(2, 1).unwrap();
        let source_rgba = [0; 4];

        assert_eq!(
            validate_rgba_buffer(&source_rgba, dimensions),
            Err(ProcessingError::InvalidBufferLength {
                expected: 8,
                actual: 4
            })
        );
    }
}
