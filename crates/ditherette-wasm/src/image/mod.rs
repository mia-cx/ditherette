//! Shared image primitives used by processing stages.

/// Validated image dimensions.
pub mod dimensions;
/// Helpers for tightly packed RGBA byte buffers.
pub mod rgba;

pub use dimensions::ImageDimensions;
