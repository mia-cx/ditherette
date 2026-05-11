//! Image resize algorithms.

pub mod nearest;

pub use nearest::{resize_rgba_nearest, resize_rgba_nearest_into};
