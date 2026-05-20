//! Image layout errors and checked validation helpers.
//!
//! Constructors for dimensions, strides, views, and buffers route through these
//! helpers so layout failures produce precise errors. Keeping this logic central
//! prevents processing modules from duplicating defensive buffer checks.

use std::{fmt, mem};

use super::{dimensions::ImageDimensions, formats::ImageFormat, stride::RowStride};

/// Errors produced when constructing image dimensions, strides, or views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageLayoutError {
    ZeroWidth,
    ZeroHeight,
    PixelCountOverflow,
    StorageLengthOverflow,
    ByteLengthOverflow,
    ChannelCountZero,
    ChannelOutOfRange {
        channel: usize,
        channel_count: usize,
    },
    StrideTooSmall {
        stride: usize,
        row_len: usize,
    },
    BufferTooShort {
        len: usize,
        required: usize,
    },
    BufferLengthMismatch {
        len: usize,
        expected: usize,
    },
}

impl fmt::Display for ImageLayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => formatter.write_str("image width must be greater than zero"),
            Self::ZeroHeight => formatter.write_str("image height must be greater than zero"),
            Self::PixelCountOverflow => formatter.write_str("image pixel count overflowed usize"),
            Self::StorageLengthOverflow => {
                formatter.write_str("image storage element length overflowed usize")
            }
            Self::ByteLengthOverflow => formatter.write_str("image byte length overflowed usize"),
            Self::ChannelCountZero => {
                formatter.write_str("image format channel count must be greater than zero")
            }
            Self::ChannelOutOfRange {
                channel,
                channel_count,
            } => write!(
                formatter,
                "image channel {channel} is outside channel count {channel_count}"
            ),
            Self::StrideTooSmall { stride, row_len } => write!(
                formatter,
                "image stride {stride} is smaller than row length {row_len}"
            ),
            Self::BufferTooShort { len, required } => write!(
                formatter,
                "image buffer length {len} is shorter than required length {required}"
            ),
            Self::BufferLengthMismatch { len, expected } => write!(
                formatter,
                "image buffer length {len} does not match expected packed length {expected}"
            ),
        }
    }
}

impl std::error::Error for ImageLayoutError {}

pub(crate) fn checked_pixel_count(dimensions: ImageDimensions) -> Result<usize, ImageLayoutError> {
    dimensions
        .width_usize()
        .checked_mul(dimensions.height_usize())
        .ok_or(ImageLayoutError::PixelCountOverflow)
}

pub(crate) fn checked_row_len<F: ImageFormat>(
    dimensions: ImageDimensions,
) -> Result<usize, ImageLayoutError> {
    if F::CHANNEL_COUNT == 0 {
        return Err(ImageLayoutError::ChannelCountZero);
    }

    dimensions
        .width_usize()
        .checked_mul(F::CHANNEL_COUNT)
        .ok_or(ImageLayoutError::StorageLengthOverflow)
}

pub(crate) fn checked_storage_len<F: ImageFormat>(
    dimensions: ImageDimensions,
) -> Result<usize, ImageLayoutError> {
    checked_pixel_count(dimensions)?
        .checked_mul(F::CHANNEL_COUNT)
        .ok_or(ImageLayoutError::StorageLengthOverflow)
}

pub(crate) fn checked_byte_len<F: ImageFormat>(
    dimensions: ImageDimensions,
) -> Result<usize, ImageLayoutError> {
    checked_storage_len::<F>(dimensions)?
        .checked_mul(mem::size_of::<F::Storage>())
        .ok_or(ImageLayoutError::ByteLengthOverflow)
}

pub(crate) fn checked_required_len<F: ImageFormat>(
    dimensions: ImageDimensions,
    stride: RowStride,
) -> Result<usize, ImageLayoutError> {
    let row_len = checked_row_len::<F>(dimensions)?;
    let height = dimensions.height_usize();
    let stride = stride.elements();

    if stride < row_len {
        return Err(ImageLayoutError::StrideTooSmall { stride, row_len });
    }

    let rows_before_last = height - 1;
    stride
        .checked_mul(rows_before_last)
        .and_then(|prefix| prefix.checked_add(row_len))
        .ok_or(ImageLayoutError::StorageLengthOverflow)
}

pub(crate) fn validate_strided_len<F: ImageFormat>(
    len: usize,
    dimensions: ImageDimensions,
    stride: RowStride,
) -> Result<(), ImageLayoutError> {
    let required = checked_required_len::<F>(dimensions, stride)?;
    if len < required {
        return Err(ImageLayoutError::BufferTooShort { len, required });
    }
    Ok(())
}

pub(crate) fn validate_packed_len<F: ImageFormat>(
    len: usize,
    dimensions: ImageDimensions,
) -> Result<(), ImageLayoutError> {
    let expected = checked_storage_len::<F>(dimensions)?;
    if len != expected {
        return Err(ImageLayoutError::BufferLengthMismatch { len, expected });
    }
    Ok(())
}

pub(crate) fn validate_channel<F: ImageFormat>(channel: usize) -> Result<(), ImageLayoutError> {
    if channel >= F::CHANNEL_COUNT {
        return Err(ImageLayoutError::ChannelOutOfRange {
            channel,
            channel_count: F::CHANNEL_COUNT,
        });
    }
    Ok(())
}
