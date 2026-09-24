//! Nearest integer coordinate mapping over the shared production resize anchors.
//!
//! This duplicates the spec anchor formulas so production kernels can evolve
//! independently without importing oracle code.

pub use crate::prod::resize::common::alignment::{AxisAlignment, ResizeAnchor};

/// Build a source-coordinate lookup table for one resize axis.
pub fn axis_coordinate_map(source_len: u32, output_len: u32, alignment: AxisAlignment) -> Vec<u32> {
    assert!(
        source_len > 0 && output_len > 0,
        "axis lengths must be non-zero"
    );
    (0..output_len)
        .map(|output_coordinate| {
            map_axis_coordinate(output_coordinate, source_len, output_len, alignment)
        })
        .collect()
}

/// Map one output coordinate to the nearest source coordinate for one axis.
///
/// The formulas are duplicated from `spec` so production kernels can build
/// coordinate maps without importing oracle code. Results are clamped to the
/// last source coordinate to cover integer edge cases at the far edge.
pub fn map_axis_coordinate(
    output_coordinate: u32,
    source_len: u32,
    output_len: u32,
    alignment: AxisAlignment,
) -> u32 {
    assert!(
        source_len > 0 && output_len > 0,
        "axis lengths must be non-zero"
    );
    match alignment {
        AxisAlignment::Start => map_start_coordinate(output_coordinate, source_len, output_len),
        AxisAlignment::Center => map_center_coordinate(output_coordinate, source_len, output_len),
        AxisAlignment::End => map_end_coordinate(output_coordinate, source_len, output_len),
    }
}

// REJECT(perf): Replacing one-time map-building u128 divisions with u64 math
// passed correctness but regressed small default nearest cases by roughly -2% to
// -4% in `ditherette-bench run nearest`.
fn map_start_coordinate(output_coordinate: u32, source_len: u32, output_len: u32) -> u32 {
    let mapped = u128::from(output_coordinate) * u128::from(source_len) / u128::from(output_len);
    mapped.min(u128::from(source_len - 1)) as u32
}

fn map_center_coordinate(output_coordinate: u32, source_len: u32, output_len: u32) -> u32 {
    let numerator = (u128::from(output_coordinate) * 2 + 1) * u128::from(source_len);
    let denominator = u128::from(output_len) * 2;
    let mapped = numerator / denominator;
    mapped.min(u128::from(source_len - 1)) as u32
}

fn map_end_coordinate(output_coordinate: u32, source_len: u32, output_len: u32) -> u32 {
    let numerator = (u128::from(output_coordinate) + 1) * u128::from(source_len) - 1;
    let mapped = numerator / u128::from(output_len);
    mapped.min(u128::from(source_len - 1)) as u32
}
