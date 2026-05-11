//! Rust/Wasm image-processing core for Ditherette.
//!
//! Browser and TypeScript code own file input and decode. This crate receives
//! canonical, tightly packed RGBA buffers plus explicit dimensions, then applies
//! deterministic image-processing stages.

pub mod error;
pub mod image;
pub mod resize;
mod wasm;

pub use wasm::{
    antialias_rgba_box3, hello, resize_rgba_area, resize_rgba_bicubic, resize_rgba_bilinear,
    resize_rgba_box, resize_rgba_lanczos2, resize_rgba_lanczos2_scale_aware, resize_rgba_lanczos3,
    resize_rgba_lanczos3_scale_aware, resize_rgba_nearest, resize_rgba_trilinear,
};
