//! Bilinear output-to-source coordinate mapping.
//!
//! These helpers preserve the spec oracle's anchor math while staying local to
//! production bilinear.

use std::ops::RangeInclusive;

use super::alignment::AxisAlignment;

// NOTE(perf): Axis position caching is handled by `BilinearResizePlan`; keep
// this helper direct-evaluated because prior-art incremental coordinate updates
// changed exact output.

pub(super) fn map_axis_position(
    output_coordinate: u32,
    source_len: u32,
    output_len: u32,
    alignment: AxisAlignment,
) -> f64 {
    let output = f64::from(output_coordinate);
    let source_len = f64::from(source_len);
    let output_len = f64::from(output_len);

    match alignment {
        AxisAlignment::Start => output * source_len / output_len,
        AxisAlignment::Center => (output + 0.5) * source_len / output_len - 0.5,
        AxisAlignment::End => (output + 1.0) * source_len / output_len - 1.0,
    }
}

// NOTE(perf): `BilinearResizePlan` stores compact nonzero support taps with
// clamped source indices and unnormalized f64 weights. Do not merge duplicate
// edge taps unless exact oracle output is revalidated.
pub(super) fn support_range(position: f64, scale: f64) -> RangeInclusive<i64> {
    let support = scale;
    (position - support).floor() as i64..=(position + support).ceil() as i64
}

// CLOSE(perf): Tap clamping now happens once during `BilinearResizePlan`
// construction, not in the hot pixel loop. Keep generic clamping here until plan
// setup itself is benchmarked as material.
pub(super) fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
