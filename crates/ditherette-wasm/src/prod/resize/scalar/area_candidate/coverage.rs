//! Coverage math for scalar production area resize.
//!
//! Area resize is defined by source-space interval overlap. These helpers keep
//! rectangle math separate from RGBA8 row traversal so optimization work can
//! precompute spans without changing the semantic rule.

/// Return how much two one-dimensional source-space intervals overlap.
pub(super) fn interval_overlap(a_start: f64, a_end: f64, b_start: f64, b_end: f64) -> f64 {
    (a_end.min(b_end) - a_start.max(b_start)).max(0.0)
}
