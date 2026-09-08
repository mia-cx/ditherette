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

/// Private pool capacity from the existing production scheduling policy.
#[cfg(feature = "threads")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = privateThreadCount)]
pub fn private_thread_count(available_parallelism: u32) -> u32 {
    prod::tiling::WorkerBudget::from_available_parallelism(available_parallelism).pool_size()
}

/// A failed dispatched startup discards its entire module memory, including this receiver.
/// Forget the builder so its JS finalizer cannot free a receiver still borrowed by a terminating worker.
#[cfg(feature = "threads")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = privateAbandonThreadPool)]
pub fn private_abandon_thread_pool(builder: wasm_bindgen_rayon::wbg_rayon_PoolBuilder) {
    std::mem::forget(builder);
}
