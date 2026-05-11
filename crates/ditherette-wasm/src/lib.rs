//! Rust/Wasm image-processing core for Ditherette.
//!
//! Browser and TypeScript code own file input and decode. This crate receives
//! canonical, tightly packed RGBA buffers plus explicit dimensions, then applies
//! deterministic image-processing stages.

pub mod error;
pub mod image;
pub mod resize;
mod wasm;

pub use wasm::{hello, resize_rgba_nearest};
