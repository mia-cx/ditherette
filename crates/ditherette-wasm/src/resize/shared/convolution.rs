/// Separable reconstruction kernel used by convolution-based resize modes.
pub(crate) trait Kernel: Copy {
    fn support(self) -> f32;
    fn weight(self, distance: f32) -> f32;
}
