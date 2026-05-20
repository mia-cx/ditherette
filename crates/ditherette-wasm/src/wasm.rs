//! Wasm-facing exports for the fresh crate.
//!
//! This file is intentionally tiny while the Rust-side spec/prod architecture is
//! being designed. Future exports should validate raw JavaScript buffers at this
//! boundary, construct typed image views, and delegate to public crate APIs.

use wasm_bindgen::prelude::*;

/// Returns a friendly greeting from the fresh Rust/Wasm module.
#[wasm_bindgen]
pub fn hello(name: &str) -> String {
    format!("Hello, {name}, from Ditherette's fresh Rust core!")
}
