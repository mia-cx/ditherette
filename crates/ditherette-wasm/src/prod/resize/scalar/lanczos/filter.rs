//! Lanczos windowed-sinc filter weights.

use std::{f64::consts::PI, num::NonZeroU32};

use super::super::convolution::ReconstructionKernel;

// TODO(perf:micro, rank=35, after perf:layout lanczos-tap-pruning): Test a
// bounded Lanczos weight approximation for plan construction, such as f32 math
// or a small sinc lookup/polynomial, because cold one-shot resizes pay this sin
// cost every call. Verify bounded correctness, then benchmark all four Lanczos
// profiles.

#[derive(Debug, Clone, Copy)]
pub(super) struct FixedLanczos<const RADIUS: u32> {
    _private: (),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Lanczos {
    radius: f64,
}

impl<const RADIUS: u32> FixedLanczos<RADIUS> {
    pub(super) const fn new() -> Self {
        assert!(RADIUS > 0, "Lanczos radius must be greater than zero");
        Self { _private: () }
    }
}

impl Lanczos {
    pub(super) fn new(radius: NonZeroU32) -> Self {
        Self {
            radius: f64::from(radius.get()),
        }
    }
}

impl<const RADIUS: u32> ReconstructionKernel for FixedLanczos<RADIUS> {
    fn radius(&self) -> f64 {
        f64::from(RADIUS)
    }

    fn weight(&self, distance: f64) -> f64 {
        lanczos_weight(distance, f64::from(RADIUS))
    }
}

impl ReconstructionKernel for Lanczos {
    fn radius(&self) -> f64 {
        self.radius
    }

    fn weight(&self, distance: f64) -> f64 {
        lanczos_weight(distance, self.radius)
    }
}

fn lanczos_weight(distance: f64, radius: f64) -> f64 {
    let x = distance.abs();
    if x == 0.0 {
        1.0
    } else if x < radius {
        sinc(x) * sinc(x / radius)
    } else {
        0.0
    }
}

fn sinc(value: f64) -> f64 {
    if value == 0.0 {
        1.0
    } else {
        let x = PI * value;
        x.sin() / x
    }
}
