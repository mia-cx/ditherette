//! Exact f64 dependencies private to the missing trilinear recipe.
//! Landed fractional area/bilinear use different accumulation precision.

pub(super) mod common {
    pub use crate::prod::resize::scalar::bilinear::alignment;
    pub mod coordinates;
    pub mod sample;
}

pub(super) mod scalar {
    pub mod area;
    pub mod bilinear;
}
