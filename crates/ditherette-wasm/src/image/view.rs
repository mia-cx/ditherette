//! Borrowed typed views over packed image storage.
//!
//! Views validate dimensions, row stride, and flat backing storage once, then
//! expose safe logical row/pixel/channel access that hides padding. The format
//! marker describes channel layout while the data remains a hot-path-friendly
//! flat array.

use std::marker::PhantomData;

use super::{
    dimensions::ImageDimensions,
    formats::ImageFormat,
    stride::RowStride,
    validate::{validate_channel, validate_packed_len, validate_strided_len, ImageLayoutError},
};

/// Immutable typed view over packed image-shaped data.
#[derive(Debug, Clone, Copy)]
pub struct ImageView<'a, F: ImageFormat> {
    data: &'a [F::Storage],
    dimensions: ImageDimensions,
    stride: RowStride,
    format: PhantomData<F>,
}

impl<'a, F: ImageFormat> ImageView<'a, F> {
    pub fn new(
        data: &'a [F::Storage],
        dimensions: ImageDimensions,
        stride: RowStride,
    ) -> Result<Self, ImageLayoutError> {
        validate_strided_len::<F>(data.len(), dimensions, stride)?;
        Ok(Self {
            data,
            dimensions,
            stride,
            format: PhantomData,
        })
    }

    pub fn packed(
        data: &'a [F::Storage],
        dimensions: ImageDimensions,
    ) -> Result<Self, ImageLayoutError> {
        validate_packed_len::<F>(data.len(), dimensions)?;
        Self::new(data, dimensions, RowStride::packed::<F>(dimensions)?)
    }

    pub const fn data(&self) -> &'a [F::Storage] {
        self.data
    }

    pub const fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    pub const fn stride(&self) -> RowStride {
        self.stride
    }

    pub fn row(&self, y: u32) -> Option<&'a [F::Storage]> {
        let y = usize::try_from(y).ok()?;
        if y >= self.dimensions.height_usize() {
            return None;
        }

        let start = y.checked_mul(self.stride.elements())?;
        let end = start.checked_add(self.row_len())?;
        self.data.get(start..end)
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<&'a [F::Storage]> {
        let x = usize::try_from(x).ok()?;
        if x >= self.dimensions.width_usize() {
            return None;
        }

        let start = x.checked_mul(F::CHANNEL_COUNT)?;
        let end = start.checked_add(F::CHANNEL_COUNT)?;
        self.row(y)?.get(start..end)
    }

    pub fn channel(&self, x: u32, y: u32, channel: usize) -> Option<F::Storage> {
        validate_channel::<F>(channel).ok()?;
        self.pixel(x, y)?.get(channel).copied()
    }

    fn row_len(&self) -> usize {
        self.dimensions.width_usize() * F::CHANNEL_COUNT
    }
}

/// Mutable typed view over packed image-shaped data.
#[derive(Debug)]
pub struct ImageViewMut<'a, F: ImageFormat> {
    data: &'a mut [F::Storage],
    dimensions: ImageDimensions,
    stride: RowStride,
    format: PhantomData<F>,
}

impl<'a, F: ImageFormat> ImageViewMut<'a, F> {
    pub fn new(
        data: &'a mut [F::Storage],
        dimensions: ImageDimensions,
        stride: RowStride,
    ) -> Result<Self, ImageLayoutError> {
        validate_strided_len::<F>(data.len(), dimensions, stride)?;
        Ok(Self {
            data,
            dimensions,
            stride,
            format: PhantomData,
        })
    }

    pub fn packed(
        data: &'a mut [F::Storage],
        dimensions: ImageDimensions,
    ) -> Result<Self, ImageLayoutError> {
        validate_packed_len::<F>(data.len(), dimensions)?;
        Self::new(data, dimensions, RowStride::packed::<F>(dimensions)?)
    }

    pub fn data(&self) -> &[F::Storage] {
        self.data
    }

    pub fn data_mut(&mut self) -> &mut [F::Storage] {
        self.data
    }

    pub const fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    pub const fn stride(&self) -> RowStride {
        self.stride
    }

    pub fn as_view(&self) -> ImageView<'_, F> {
        ImageView::new(self.data, self.dimensions, self.stride)
            .expect("validated mutable image view should produce valid immutable view")
    }

    pub fn row(&self, y: u32) -> Option<&[F::Storage]> {
        self.as_view().row(y)
    }

    pub fn row_mut(&mut self, y: u32) -> Option<&mut [F::Storage]> {
        let y = usize::try_from(y).ok()?;
        if y >= self.dimensions.height_usize() {
            return None;
        }

        let start = y.checked_mul(self.stride.elements())?;
        let end = start.checked_add(self.row_len())?;
        self.data.get_mut(start..end)
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<&[F::Storage]> {
        self.as_view().pixel(x, y)
    }

    pub fn pixel_mut(&mut self, x: u32, y: u32) -> Option<&mut [F::Storage]> {
        let x = usize::try_from(x).ok()?;
        if x >= self.dimensions.width_usize() {
            return None;
        }

        let start = x.checked_mul(F::CHANNEL_COUNT)?;
        let end = start.checked_add(F::CHANNEL_COUNT)?;
        self.row_mut(y)?.get_mut(start..end)
    }

    pub fn channel(&self, x: u32, y: u32, channel: usize) -> Option<F::Storage> {
        self.as_view().channel(x, y, channel)
    }

    pub fn set_channel(&mut self, x: u32, y: u32, channel: usize, value: F::Storage) -> Option<()> {
        validate_channel::<F>(channel).ok()?;
        let pixel = self.pixel_mut(x, y)?;
        *pixel.get_mut(channel)? = value;
        Some(())
    }

    fn row_len(&self) -> usize {
        self.dimensions.width_usize() * F::CHANNEL_COUNT
    }
}
