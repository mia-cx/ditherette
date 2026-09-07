//! Numeric sample conversion for resize specs.
//!
//! Resampling filters accumulate channels as `f64` so the spec code can express
//! the math directly. This trait defines how flat storage elements enter and
//! leave that semantic accumulator without introducing per-pixel structs.

/// Storage elements that can participate in exact resize spec accumulation.
pub trait ResizeSample: Copy + Default + 'static {
    fn to_f64(self) -> f64;
    fn from_f64(value: f64) -> Self;
}

impl ResizeSample for u8 {
    fn to_f64(self) -> f64 {
        f64::from(self)
    }

    fn from_f64(value: f64) -> Self {
        value.clamp(0.0, 255.0).round() as u8
    }
}

impl ResizeSample for f32 {
    fn to_f64(self) -> f64 {
        f64::from(self)
    }

    fn from_f64(value: f64) -> Self {
        value as f32
    }
}
