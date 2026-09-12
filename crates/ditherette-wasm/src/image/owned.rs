//! Owned typed image buffers.
//!
//! `ImageBuf` couples a flat `Vec<F::Storage>` with a format marker, validated
//! dimensions, and stride so callers can allocate outputs or store intermediate
//! packed images. It reuses the same view validation path as borrowed images to
//! keep owned and borrowed semantics identical.

use std::marker::PhantomData;

use super::{
    dimensions::ImageDimensions,
    formats::ImageFormat,
    stride::RowStride,
    validate::{validate_packed_len, validate_strided_len, ImageLayoutError},
    view::{ImageView, ImageViewMut},
};

/// Owned packed image buffer for format `F`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageBuf<F: ImageFormat> {
    data: Vec<F::Storage>,
    dimensions: ImageDimensions,
    stride: RowStride,
    format: PhantomData<F>,
}

impl<F: ImageFormat> ImageBuf<F> {
    pub fn new_packed(dimensions: ImageDimensions) -> Result<Self, ImageLayoutError> {
        let len = dimensions.storage_len::<F>()?;
        Ok(Self {
            data: vec![F::Storage::default(); len],
            dimensions,
            stride: RowStride::packed::<F>(dimensions)?,
            format: PhantomData,
        })
    }

    pub fn from_vec_packed(
        data: Vec<F::Storage>,
        dimensions: ImageDimensions,
    ) -> Result<Self, ImageLayoutError> {
        validate_packed_len::<F>(data.len(), dimensions)?;
        Ok(Self {
            data,
            dimensions,
            stride: RowStride::packed::<F>(dimensions)?,
            format: PhantomData,
        })
    }

    pub fn from_vec_strided(
        data: Vec<F::Storage>,
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

    pub fn as_view(&self) -> ImageView<'_, F> {
        ImageView::new(&self.data, self.dimensions, self.stride)
            .expect("validated owned image buffer should produce valid immutable view")
    }

    pub fn as_view_mut(&mut self) -> ImageViewMut<'_, F> {
        ImageViewMut::new(&mut self.data, self.dimensions, self.stride)
            .expect("validated owned image buffer should produce valid mutable view")
    }

    pub const fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    pub const fn stride(&self) -> RowStride {
        self.stride
    }

    pub fn data(&self) -> &[F::Storage] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [F::Storage] {
        &mut self.data
    }

    pub fn into_vec(self) -> Vec<F::Storage> {
        self.data
    }
}
