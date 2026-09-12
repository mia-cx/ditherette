//! Spec resize algorithms and coordinate semantics.
//!
//! Resize specs favor direct coordinate mapping and obvious loops. They are the
//! byte-for-byte oracles that future production scalar/SIMD/tiled resize paths
//! must match for exact modes.

pub mod common;
pub mod scalar;
