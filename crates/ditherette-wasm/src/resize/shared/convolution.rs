/// Separable reconstruction kernel used by convolution-based resize modes.
// TODO(perf): Add optional associated constants for fixed support/tap count so
// convolution can dispatch to fixed-size planners and hot loops without calling
// trait methods in monomorphic kernels. Benchmark with `pnpm bench:resize:bicubic`
// and `pnpm bench:resize:lanczos3` before accepting.
pub(crate) trait Kernel: Copy {
    fn support(self) -> f32;
    fn weight(self, distance: f32) -> f32;
}
