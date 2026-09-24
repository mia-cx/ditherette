//! Production resize anchors and continuous coordinate mapping.
//!
//! This duplicates the spec anchor vocabulary so production filters stay
//! independent from the oracle while preserving identical coordinate math.
//! Every production filter shares these types; nearest keeps its own integer
//! coordinate formulas next to its kernels.

use std::ops::RangeInclusive;

/// One-dimensional anchor used when mapping output coordinates to source positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisAlignment {
    /// Align the start edge of each axis.
    Start,
    /// Align pixel centers.
    Center,
    /// Align the end edge of each axis.
    End,
}

/// Two-dimensional resize anchor composed from x/y axis alignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeAnchor {
    /// Align both axes at their start edges.
    TopLeft,
    /// Center horizontally and align the vertical axis at the start edge.
    Top,
    /// Align horizontally at the end edge and vertically at the start edge.
    TopRight,
    /// Align the horizontal axis at the start edge and center vertically.
    Left,
    /// Align pixel centers on both axes.
    #[default]
    Center,
    /// Align the horizontal axis at the end edge and center vertically.
    Right,
    /// Align horizontally at the start edge and vertically at the end edge.
    BottomLeft,
    /// Center horizontally and align the vertical axis at the end edge.
    Bottom,
    /// Align both axes at their end edges.
    BottomRight,
}

impl ResizeAnchor {
    /// Return the horizontal and vertical axis alignments for this anchor.
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

// NOTE(perf): Filter plans cache axis positions; keep this helper direct-evaluated
// because prior-art incremental coordinate updates changed exact output.

/// Maps one output coordinate to a continuous source pixel-index position.
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

/// Integer source coordinates within `support` of `position`, before weights and clamping.
pub fn support_range(position: f64, support: f64) -> RangeInclusive<i64> {
    (position - support).floor() as i64..=(position + support).ceil() as i64
}
