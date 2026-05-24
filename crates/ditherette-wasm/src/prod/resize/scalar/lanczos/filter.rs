//! Lanczos windowed-sinc filter weights.

use std::f64::consts::PI;

use super::super::convolution::ReconstructionKernel;

// TODO(perf:micro, rank=35, after perf:layout lanczos-tap-pruning): Test a
// bounded Lanczos weight approximation for plan construction, such as f32 math
// or a small sinc lookup/polynomial, because cold one-shot resizes pay this sin
// cost every call. Verify bounded correctness, then benchmark all four Lanczos
// profiles.

#[derive(Debug, Clone, Copy)]
pub(super) struct Lanczos {
    radius: f64,
}

impl Lanczos {
    pub(super) fn new(radius: u32) -> Self {
        assert!(radius > 0, "Lanczos radius must be greater than zero");
        Self {
            radius: f64::from(radius),
        }
    }
}

impl ReconstructionKernel for Lanczos {
    fn radius(&self) -> f64 {
        self.radius
    }

    fn weight(&self, distance: f64) -> f64 {
        let x = distance.abs();
        if x == 0.0 {
            1.0
        } else if x < self.radius {
            sinc(x) * sinc(x / self.radius)
        } else {
            0.0
        }
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
