//! Scalar resize spec implementations.
//!
//! Scalar specs are direct, single-threaded executable oracles. They do not
//! precompute production plans, schedule tiling, or optimize hot paths.

pub mod area;
pub mod bicubic;
pub mod bilinear;
pub mod convolution;
pub mod lanczos;
pub mod nearest;
pub mod trilinear;
