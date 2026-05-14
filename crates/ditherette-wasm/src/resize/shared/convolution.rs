/// Separable reconstruction kernel used by convolution-based resize modes.
pub(crate) trait Kernel: Copy {
    fn radius(self) -> f64;
    fn weight(self, distance: f64) -> f64;
}
