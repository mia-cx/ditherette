//! Private adapters for the selected module's worker pool.

use wasm_bindgen::prelude::*;

/// Private pool capacity from the existing production scheduling policy.
#[wasm_bindgen(js_name = privateThreadCount)]
pub fn private_thread_count(available_parallelism: u32) -> u32 {
    crate::prod::tiling::WorkerBudget::from_available_parallelism(available_parallelism).pool_size()
}

/// A failed dispatched startup discards its entire module memory, including this receiver.
/// Forget the builder so its JS finalizer cannot free a receiver still borrowed by a terminating worker.
#[wasm_bindgen(js_name = privateAbandonThreadPool)]
pub fn private_abandon_thread_pool(builder: wasm_bindgen_rayon::wbg_rayon_PoolBuilder) {
    std::mem::forget(builder);
}
