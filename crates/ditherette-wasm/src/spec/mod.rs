//! Executable specifications for Ditherette image-processing domains.
//!
//! Code under `spec/` defines simple, readable oracle behavior. Production code
//! may optimize or tile these operations later, but exact modes must preserve
//! the semantics expressed here.

pub mod adapters;
pub mod color;
pub mod contract;
pub mod dither;
pub mod palette;
pub mod quantize;
pub mod resize;
pub mod tiling;
