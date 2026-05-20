//! Production resize implementations.
//!
//! Production resize code is allowed to precompute plans, specialize kernels,
//! and later add tiling/SIMD paths while staying byte-for-byte compatible with
//! the public spec oracles for exact modes.

pub mod common;
pub mod scalar;
