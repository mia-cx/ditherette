//! Image resize algorithms.

pub mod antialias;
pub mod area;
pub mod bicubic;
pub mod bilinear;
pub mod r#box;
mod buffers;
mod convolution;
mod convolution_reference;
mod lanczos;
pub mod lanczos2;
pub mod lanczos2_scale_aware;
pub mod lanczos3;
pub mod lanczos3_scale_aware;
pub mod nearest;
pub mod trilinear;

pub use antialias::{antialias_rgba_box3, antialias_rgba_box3_into};
pub use area::{resize_rgba_area, resize_rgba_area_into};
pub use bicubic::{resize_rgba_bicubic, resize_rgba_bicubic_into};
pub use bilinear::{resize_rgba_bilinear, resize_rgba_bilinear_into};
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
