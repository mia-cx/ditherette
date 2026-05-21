//! Triangle-filter weights for bilinear resize.

// TODO(perf:micro, rank=23, after perf:layout bilinear-nonzero-taps): If the
// direct filter remains hot after planned weights land, reduce this helper to a
// single `abs()` plus clamp-style expression or inline it into tap generation.
// Verify byte-exact output with `--oracle spec:resize:bilinear:scalar`, then
// benchmark `ditherette-bench run bilinear --baseline accepted`.
pub(super) fn triangle_weight(distance: f64) -> f64 {
    if distance.abs() < 1.0 {
        1.0 - distance.abs()
    } else {
        0.0
    }
}
