//! Production implementations for optimized image-processing paths.
//!
//! Code under `prod` owns performance-oriented implementations. It may duplicate
//! spec formulas to preserve independence, but it must not import `crate::spec`.

pub mod color;
pub mod contract;
pub mod palette;
pub mod pipeline;
pub mod quantize;
pub mod resize;
pub mod tiling;
