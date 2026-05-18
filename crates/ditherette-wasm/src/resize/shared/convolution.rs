/// Separable reconstruction kernel used by convolution-based resize modes.
pub(crate) trait Kernel: Copy {
    const FIXED_SUPPORT: Option<f32> = None;
    const FIXED_UPSCALE_TAP_COUNT: Option<usize> = None;

    fn support(self) -> f32;
    fn weight(self, distance: f32) -> f32;
}
