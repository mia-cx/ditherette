//! Production resize implementations.
//!
//! Production resize code is allowed to precompute plans, specialize kernels,
//! and later add tiling/SIMD paths while staying byte-for-byte compatible with
//! the public spec oracles for exact modes.
//!
//! Ditherette production resize operates on normalized packed `Rgba8` images.
//! HDR or non-RGBA inputs should be converted at the boundary before they reach
//! these kernels. The readable `spec` implementations may stay generic, but
//! production resize should prefer explicit `Rgba8` paths over format-generic
//! hot loops.

pub mod common;
pub mod scalar;
