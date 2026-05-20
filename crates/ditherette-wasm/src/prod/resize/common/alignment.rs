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
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl ResizeAnchor {
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

// TODO(perf:api, rank=1): Add a caller-owned nearest resize plan that caches
// axis maps across repeated same-shape resizes, so benchmark batches and future
// UI preview loops do not allocate/recompute coordinates per image. Verify with
// `ditherette-bench run nearest --oracle spec:resize:nearest:scalar`; benchmark
// with `ditherette-bench run nearest --baseline accepted`.
/// Build a source-coordinate lookup table for one resize axis.
pub fn axis_coordinate_map(source_len: u32, output_len: u32, alignment: AxisAlignment) -> Vec<u32> {
    (0..output_len)
        .map(|output_coordinate| {
            map_axis_coordinate(output_coordinate, source_len, output_len, alignment)
        })
        .collect()
}

pub fn map_axis_coordinate(
    output_coordinate: u32,
    source_len: u32,
    output_len: u32,
    alignment: AxisAlignment,
) -> u32 {
    match alignment {
        AxisAlignment::Start => map_start_coordinate(output_coordinate, source_len, output_len),
        AxisAlignment::Center => map_center_coordinate(output_coordinate, source_len, output_len),
        AxisAlignment::End => map_end_coordinate(output_coordinate, source_len, output_len),
    }
}

// REJECT(perf): Replacing one-time map-building u128 divisions with u64 math
// passed correctness but regressed small default nearest cases by roughly -2% to
// -4% in `ditherette-bench run nearest --baseline accepted`.
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
