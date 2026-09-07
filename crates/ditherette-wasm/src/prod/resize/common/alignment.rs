//! Resize alignment semantics.
//!
//! Alignment is expressed as the anchor point used on each axis, not as a vague
//! "corner" mode. The 3x3 `ResizeAnchor` presets make runtime alignment choices
//! explicit while the axis mapper keeps the coordinate formulas auditable.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_map_to_axis_alignments() {
        assert_eq!(
            ResizeAnchor::TopLeft.axes(),
            (AxisAlignment::Start, AxisAlignment::Start)
        );
        assert_eq!(
            ResizeAnchor::Center.axes(),
            (AxisAlignment::Center, AxisAlignment::Center)
        );
        assert_eq!(
            ResizeAnchor::BottomRight.axes(),
            (AxisAlignment::End, AxisAlignment::End)
        );
    }

    #[test]
    fn axis_alignment_selects_start_center_or_end_samples() {
        assert_eq!(mapping::<3>(4, 3, AxisAlignment::Start), [0, 1, 2]);
        assert_eq!(mapping::<3>(4, 3, AxisAlignment::Center), [0, 2, 3]);
        assert_eq!(mapping::<3>(4, 3, AxisAlignment::End), [1, 2, 3]);
    }

    #[test]
    fn axis_alignment_clamps_to_single_source_pixel() {
        assert_eq!(mapping::<4>(1, 4, AxisAlignment::Start), [0, 0, 0, 0]);
        assert_eq!(mapping::<4>(1, 4, AxisAlignment::Center), [0, 0, 0, 0]);
        assert_eq!(mapping::<4>(1, 4, AxisAlignment::End), [0, 0, 0, 0]);
    }

    fn mapping<const N: usize>(
        source_len: u32,
        output_len: u32,
        alignment: AxisAlignment,
    ) -> [u32; N] {
        std::array::from_fn(|index| {
            map_axis_coordinate(index as u32, source_len, output_len, alignment)
        })
    }
}
