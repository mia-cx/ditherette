use crate::resize::shared::convolution::Kernel;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Bicubic;

impl Kernel for Bicubic {
    fn radius(self) -> f64 {
        2.0
    }

    fn weight(self, distance: f64) -> f64 {
        let x = distance.abs();
        let a = -0.5;

        if x < 1.0 {
            (a + 2.0) * x.powi(3) - (a + 3.0) * x.powi(2) + 1.0
        } else if x < 2.0 {
            a * x.powi(3) - 5.0 * a * x.powi(2) + 8.0 * a * x - 4.0 * a
        } else {
            0.0
        }
    }
}
