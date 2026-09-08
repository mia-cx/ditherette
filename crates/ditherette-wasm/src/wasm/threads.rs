//! Private adapters for the selected module's worker pool.

use wasm_bindgen::prelude::*;

/// Private pool capacity from the existing production scheduling policy.
#[wasm_bindgen(js_name = privateThreadCount)]
pub fn private_thread_count(available_parallelism: u32) -> u32 {
    crate::prod::tiling::WorkerBudget::from_available_parallelism(available_parallelism).pool_size()
}
