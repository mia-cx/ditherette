//! Exact f64 area and bilinear stages private to production trilinear.
//! Landed fractional area/bilinear use different accumulation precision.

pub(super) mod common {
    pub use crate::prod::resize::common::alignment;
    pub mod sample;
}

pub(super) mod scalar {
    pub mod area;
    pub mod bilinear;
}
