//! Production convolution alignment helpers.
//!
//! This duplicates the spec anchor vocabulary so production convolution filters
//! stay independent from the oracle while preserving identical coordinate math.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Default for ResizeAnchor {
    fn default() -> Self {
        Self::Center
    }
}
