//! Continuous resize coordinate mapping.
//!
//! Continuous filters sample around a source-space position rather than picking
//! a single integer coordinate directly. This file maps output coordinates into
//! source pixel-index space using the same axis-anchor vocabulary as nearest.

use super::alignment::AxisAlignment;

/// Maps one output coordinate to a continuous source pixel-index position.
///
/// Source pixel centers live at integer coordinates `0, 1, 2, ...`.
pub fn map_axis_position(
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
