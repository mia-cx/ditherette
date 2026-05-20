//! Marker trait for scalar storage elements used by packed image formats.
//!
//! Formats define channel layout separately from storage. This trait only says a
//! scalar type can safely live in an image buffer; it does not attach color-space
//! or pixel semantics to the scalar itself.

/// Scalar storage element allowed in packed image buffers.
pub trait StorageElement: Copy + Default + 'static {}

impl StorageElement for u8 {}
impl StorageElement for f32 {}
