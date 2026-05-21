//! Production resize implementations.
//!
//! Production resize code is allowed to precompute plans, specialize kernels,
//! and later add tiling/SIMD paths while staying byte-for-byte compatible with
//! the public spec oracles for exact modes.
//!
//! Ditherette production resize operates only on normalized packed `Rgba8`
//! images. HDR, non-RGBA, padded-row, or subimage inputs must be converted into
//! packed RGBA8 at the boundary before they reach these kernels. See the local
//! `README.md` for the grep-friendly prod resize rules. The readable `spec`
//! implementations may stay generic; production resize is explicitly RGBA8-only.
//! Nearest additionally word-copies RGBA8 pixels internally; other filters should
//! still compute channels normally unless benchmarks prove a safe packed-word
//! strategy.

pub mod common;
pub mod scalar;
