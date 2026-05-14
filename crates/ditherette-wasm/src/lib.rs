//! Rust/Wasm image-processing core for Ditherette.
//!
//! Browser and TypeScript code own file input and decode. This crate receives
//! canonical, tightly packed RGBA buffers plus explicit dimensions, then applies
//! deterministic image-processing stages.

/// Error types shared by Rust and Wasm-facing processing code.
pub mod error;
/// Image buffer dimensions and RGBA buffer helpers.
pub mod image;
/// Resize and antialiasing filters for tightly packed RGBA buffers.
pub mod resize;
mod wasm;

pub use wasm::{
    antialias_rgba_box3, hello, resize_rgba_area, resize_rgba_bicubic, resize_rgba_bilinear,
    resize_rgba_box, resize_rgba_lanczos, resize_rgba_lanczos2, resize_rgba_lanczos2_scale_aware,
    resize_rgba_lanczos3, resize_rgba_lanczos3_scale_aware, resize_rgba_nearest,
    resize_rgba_trilinear,
};
