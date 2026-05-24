//! Production resize alignment helpers.
//!
//! This duplicates the spec anchor formulas so production kernels can evolve
//! independently without importing oracle code.

/// One-dimensional anchor used when mapping output coordinates to source coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisAlignment {
    /// Align the start edge of each axis.
    Start,
    /// Align pixel centers. This is the old Ditherette nearest behavior.
    Center,
    /// Align the end edge of each axis.
    End,
}

/// Two-dimensional resize anchor composed from x/y axis alignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAnchor {
    /// Align source and output top-left corners.
    TopLeft,
    /// Center horizontally and align top edges.
    Top,
    /// Align source and output top-right corners.
    TopRight,
    /// Align left edges and center vertically.
    Left,
    /// Align pixel centers on both axes.
    Center,
    /// Align right edges and center vertically.
    Right,
    /// Align source and output bottom-left corners.
    BottomLeft,
    /// Center horizontally and align bottom edges.
    Bottom,
    /// Align source and output bottom-right corners.
    BottomRight,
}

impl ResizeAnchor {
    /// Decompose the two-dimensional anchor into horizontal and vertical alignments.
    pub const fn axes(self) -> (AxisAlignment, AxisAlignment) {
        match self {
            Self::TopLeft => (AxisAlignment::Start, AxisAlignment::Start),
            Self::Top => (AxisAlignment::Center, AxisAlignment::Start),
            Self::TopRight => (AxisAlignment::End, AxisAlignment::Start),
            Self::Left => (AxisAlignment::Start, AxisAlignment::Center),
            Self::Center => (AxisAlignment::Center, AxisAlignment::Center),
            Self::Right => (AxisAlignment::End, AxisAlignment::Center),
            Self::BottomLeft => (AxisAlignment::Start, AxisAlignment::End),
            Self::Bottom => (AxisAlignment::Center, AxisAlignment::End),
            Self::BottomRight => (AxisAlignment::End, AxisAlignment::End),
        }
    }
}

impl Default for ResizeAnchor {
    fn default() -> Self {
        Self::Center
    }
}

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
