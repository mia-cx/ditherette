//! Catmull-Rom bicubic filter weights.

use super::super::convolution::ReconstructionKernel;

pub(super) const CATMULL_ROM: CatmullRom = CatmullRom;

#[derive(Debug, Clone, Copy)]
pub(super) struct CatmullRom;

impl ReconstructionKernel for CatmullRom {
    fn radius(&self) -> f64 {
        2.0
    }

    fn weight(&self, distance: f64) -> f64 {
        cubic_weight(distance, -0.5)
    }
}

fn cubic_weight(distance: f64, a: f64) -> f64 {
    let x = distance.abs();

    if x < 1.0 {
        (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0
    } else if x < 2.0 {
        a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a
    } else {
        0.0
    }
}
