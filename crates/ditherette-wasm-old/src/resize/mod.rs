//! Image resize algorithms.

/// Small box-filter antialiasing helpers.
pub mod antialias;
/// Exact area resampling for coverage-preserving resize.
pub mod area;
/// Bilinear RGBA resize filter.
pub mod bilinear;
/// Box-filter resize, implemented as the area filter.
pub mod r#box;
mod buffers;
#[cfg(feature = "tiling")]
#[doc(hidden)]
pub mod cpu_tiling;
/// Nearest-neighbor RGBA resize filter.
pub mod nearest;
mod reference;
mod scalar;
#[cfg(feature = "tiling")]
mod tiling;
/// Trilinear RGBA resize filter.
pub mod trilinear;

pub use antialias::{antialias_rgba_box3, antialias_rgba_box3_into};
pub use area::{resize_rgba_area, resize_rgba_area_into};
pub use bilinear::{resize_rgba_bilinear, resize_rgba_bilinear_into};
pub use nearest::{resize_rgba_nearest, resize_rgba_nearest_into};
pub use r#box::{resize_rgba_box, resize_rgba_box_into};
pub use trilinear::{resize_rgba_trilinear, resize_rgba_trilinear_into};
