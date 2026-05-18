use crate::resize::shared::convolution::Kernel;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Bicubic;

impl Kernel for Bicubic {
    fn support(self) -> f32 {
        2.0
    }

    fn weight(self, distance: f32) -> f32 {
        let x = distance.abs();
        let b = 0.0;
        let c = 0.5;

        let x2 = x * x;
        let x3 = x2 * x;
        let weight = if x < 1.0 {
            (12.0 - 9.0 * b - 6.0 * c) * x3 + (-18.0 + 12.0 * b + 6.0 * c) * x2 + (6.0 - 2.0 * b)
        } else if x < 2.0 {
            (-b - 6.0 * c) * x3
                + (6.0 * b + 30.0 * c) * x2
                + (-12.0 * b - 48.0 * c) * x
                + (8.0 * b + 24.0 * c)
        } else {
            0.0
        };

        weight / 6.0
    }
}
