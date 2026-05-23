//! Convolution output-to-source coordinate mapping.
//!
//! These helpers preserve the spec oracle's anchor math while staying local to
//! production convolution.

use std::ops::RangeInclusive;

use super::alignment::AxisAlignment;

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

pub(super) fn support_range(position: f64, support: f64) -> RangeInclusive<i64> {
    (position - support).floor() as i64..=(position + support).ceil() as i64
}

pub(super) fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
