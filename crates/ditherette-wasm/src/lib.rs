//! Fresh Rust/Wasm image-processing core for Ditherette.
//!
//! This crate intentionally starts small. The legacy implementation lives in
//! `crates/ditherette-wasm-old` while the new API, reference model, and
//! optimization boundaries are specified cleanly here.

#[cfg(feature = "bench-subjects")]
pub mod bench_subjects;
pub mod image;
pub mod prod;
pub mod spec;
mod wasm;

#[cfg(feature = "bench-subjects")]
pub use bench_subjects::bench_subjects;
pub use wasm::hello;
