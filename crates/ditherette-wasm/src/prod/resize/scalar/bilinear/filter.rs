//! Triangle-filter weights for bilinear resize.

// CLOSE(perf): After `BilinearResizePlan` started caching support taps, this
// helper runs during plan construction instead of the hot pixel loop. Keep the
// branchy direct form for readability until plan setup appears in profiling.
pub(super) fn triangle_weight(distance: f64) -> f64 {
    if distance.abs() < 1.0 {
        1.0 - distance.abs()
    } else {
        0.0
    }
}
