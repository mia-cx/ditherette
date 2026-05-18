use crate::resize::shared::convolution::Kernel;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Bicubic;

impl Kernel for Bicubic {
    const FIXED_SUPPORT: Option<f32> = Some(2.0);
    const FIXED_UPSCALE_TAP_COUNT: Option<usize> = Some(4);

    fn support(self) -> f32 {
        2.0
    }

    fn weight(self, distance: f32) -> f32 {
        let x = distance.abs();
        let b = 0.0;
        let c = 0.5;

        // TODO(perf): Replace the image-compatible f32 `powi` Catmull-Rom
        // evaluation with explicit polynomial multiplies. Benchmark with
        // `pnpm bench:resize:bicubic` before accepting.
        let weight = if x < 1.0 {
            (12.0 - 9.0 * b - 6.0 * c) * x.powi(3)
                + (-18.0 + 12.0 * b + 6.0 * c) * x.powi(2)
                + (6.0 - 2.0 * b)
        } else if x < 2.0 {
            (-b - 6.0 * c) * x.powi(3)
                + (6.0 * b + 30.0 * c) * x.powi(2)
                + (-12.0 * b - 48.0 * c) * x
                + (8.0 * b + 24.0 * c)
        } else {
            0.0
        };

        weight / 6.0
    }
}
