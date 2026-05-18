/// Separable reconstruction kernel used by convolution-based resize modes.
// REJECT(perf): Optional associated constants for fixed support/tap count
// preserved correctness but baseline refresh showed broad bicubic downscale
// regressions; keep dynamic support until a dedicated fixed planner uses it.
pub(crate) trait Kernel: Copy {
    fn support(self) -> f32;
    fn weight(self, distance: f32) -> f32;
}
