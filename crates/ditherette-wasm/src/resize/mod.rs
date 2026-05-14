//! Image resize algorithms.

/// Small box-filter antialiasing helpers.
pub mod antialias;
/// Exact area resampling for coverage-preserving resize.
pub mod area;
/// Bicubic RGBA resize filter.
pub mod bicubic;
/// Bilinear RGBA resize filter.
pub mod bilinear;
/// Box-filter resize, implemented as the area filter.
pub mod r#box;
mod buffers;
#[cfg(feature = "tiling")]
#[doc(hidden)]
pub mod cpu_tiling;
/// Generic Lanczos RGBA resize filter.
pub mod lanczos;
/// Lanczos2 RGBA resize filter.
pub mod lanczos2;
/// Scale-aware Lanczos2 RGBA resize filter.
pub mod lanczos2_scale_aware;
/// Lanczos3 RGBA resize filter.
pub mod lanczos3;
/// Scale-aware Lanczos3 RGBA resize filter.
pub mod lanczos3_scale_aware;
/// Nearest-neighbor RGBA resize filter.
pub mod nearest;
mod reference;
mod scalar;
mod shared;
#[cfg(feature = "tiling")]
mod tiling;
/// Trilinear RGBA resize filter.
pub mod trilinear;

pub use antialias::{antialias_rgba_box3, antialias_rgba_box3_into};
pub use area::{resize_rgba_area, resize_rgba_area_into};
pub use bicubic::{resize_rgba_bicubic, resize_rgba_bicubic_into};
pub use bilinear::{resize_rgba_bilinear, resize_rgba_bilinear_into};
pub use lanczos::{resize_rgba_lanczos, resize_rgba_lanczos_into};
pub use lanczos2::{resize_rgba_lanczos2, resize_rgba_lanczos2_into};
pub use lanczos2_scale_aware::{
    resize_rgba_lanczos2_scale_aware, resize_rgba_lanczos2_scale_aware_into,
};
pub use lanczos3::{resize_rgba_lanczos3, resize_rgba_lanczos3_into};
pub use lanczos3_scale_aware::{
    resize_rgba_lanczos3_scale_aware, resize_rgba_lanczos3_scale_aware_into,
};
pub use nearest::{resize_rgba_nearest, resize_rgba_nearest_into};
pub use r#box::{resize_rgba_box, resize_rgba_box_into};
pub use trilinear::{resize_rgba_trilinear, resize_rgba_trilinear_into};
