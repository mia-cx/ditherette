//! Minimal image-shaped storage types.
//!
//! This module owns representation logic: dimensions, stride validation, packed
//! format markers, borrowed views, and owned buffers. It intentionally does not
//! transform image content or destructure flat channel arrays into per-pixel
//! structs.

pub mod dimensions;
pub mod formats;
pub mod owned;
pub mod pixel;
pub mod stride;
pub mod validate;
pub mod view;

pub use dimensions::ImageDimensions;
pub use formats::{
    ImageFormat, LinearRgb32, LinearRgba32, Oklab32, Oklaba32, PaletteIndex8, Rgb8, Rgba8,
};
pub use owned::ImageBuf;
pub use pixel::StorageElement;
pub use stride::RowStride;
pub use validate::ImageLayoutError;
pub use view::{ImageView, ImageViewMut};
