/// Separable reconstruction kernel used by convolution-based resize modes.
// TODO(perf): Add optional associated constants for fixed radius/tap count so
// production convolution can dispatch to fixed-size loops without repeatedly
// calling `support()`. Benchmark with `pnpm bench:resize:bicubic` and
// `pnpm bench:resize:lanczos3` before accepting.
pub(crate) trait Kernel: Copy {
    fn support(self) -> f32;
    fn weight(self, distance: f32) -> f32;
}
