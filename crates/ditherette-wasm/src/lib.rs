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
pub use wasm::processor::{
    private_dispose, private_error_path, private_initialize, private_memory_overhead,
    private_resize,
};
pub use wasm::{
    benchmark_color_space, benchmark_resize_rgba8, convert_color_space, hello, process_rgba8,
    resize_rgba8,
};
#[cfg(feature = "threads")]
pub use wasm_bindgen_rayon::init_thread_pool;
