//! Exact nearest candidate derived from the S19 literal-copy baseline.
//!
//! Coordinates retain the copied anchor formulas, replacing repeated division
//! with quotient/remainder additions. The accepted `nearest` kernel is unchanged.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::common::alignment::{AxisAlignment, ResizeAnchor},
};

/// Resize by exact anchored nearest sampling without allocating coordinate plans.
/// Copies only logical row elements, preserving padding and every storage bit.
pub fn resize_nearest_into<F: ImageFormat>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let (x_alignment, y_alignment) = anchor.axes();
    let x_start = Axis::new(
        source_dimensions.width(),
        output_dimensions.width(),
        x_alignment,
    );
    let mut y = Axis::new(
        source_dimensions.height(),
        output_dimensions.height(),
        y_alignment,
    );

    for output_y in 0..output_dimensions.height() {
        let source_row = source
            .row(y.coordinate())
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        if source_dimensions.width() == output_dimensions.width() {
            output_row.copy_from_slice(source_row);
        } else {
            let mut x = x_start;
            for output_pixel in output_row.chunks_exact_mut(F::CHANNEL_COUNT) {
                let source_start = x.coordinate() as usize * F::CHANNEL_COUNT;
                output_pixel
                    .copy_from_slice(&source_row[source_start..source_start + F::CHANNEL_COUNT]);
                x.advance();
            }
        }
        y.advance();
    }
}

#[derive(Clone, Copy)]
struct Axis {
    quotient: u64,
    remainder: u64,
    step_quotient: u64,
    step_remainder: u64,
    denominator: u64,
    last: u64,
}

impl Axis {
    fn new(source_len: u32, output_len: u32, alignment: AxisAlignment) -> Self {
        let source = u64::from(source_len);
        let output = u64::from(output_len);
        let (initial, step, denominator) = match alignment {
            AxisAlignment::Start => (0, source, output),
            AxisAlignment::Center => (source, source * 2, output * 2),
            AxisAlignment::End => (source - 1, source, output),
        };
        Self {
            quotient: initial / denominator,
            remainder: initial % denominator,
            step_quotient: step / denominator,
            step_remainder: step % denominator,
            denominator,
            last: source - 1,
        }
    }

    fn coordinate(self) -> u32 {
        self.quotient.min(self.last) as u32
    }

    fn advance(&mut self) {
        self.quotient += self.step_quotient;
        self.remainder += self.step_remainder;
        // Both remainders are below denominator, so exactly one carry suffices.
        // Even the center denominator and summed remainders fit within 34 bits.
        if self.remainder >= self.denominator {
            self.remainder -= self.denominator;
            self.quotient += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prod::resize::common::alignment::map_axis_coordinate;

    const ALIGNMENTS: [AxisAlignment; 3] = [
        AxisAlignment::Start,
        AxisAlignment::Center,
        AxisAlignment::End,
    ];

    #[test]
    fn recurrence_matches_literal_mapper_for_small_dimensions() {
        for source in 1..=65 {
            for output in 1..=65 {
                for alignment in ALIGNMENTS {
                    let mut axis = Axis::new(source, output, alignment);
                    for coordinate in 0..output {
                        assert_eq!(
                            axis.coordinate(),
                            map_axis_coordinate(coordinate, source, output, alignment)
                        );
                        axis.advance();
                    }
                }
            }
        }
    }

    #[test]
    fn recurrence_matches_literal_mapper_near_u32_limits() {
        let lengths = [1, 2, 3, 65_537, u32::MAX / 2, u32::MAX - 1, u32::MAX];
        for source in lengths {
            for output in lengths {
                for alignment in ALIGNMENTS {
                    for offset in [0, output / 2, output.saturating_sub(8)] {
                        let mut axis = Axis::new(source, output, alignment);
                        // Seek the rational numerator, not billions of loop iterations.
                        // u128 is needed only here for the near-limit center product.
                        let initial = u128::from(axis.quotient * axis.denominator + axis.remainder);
                        let step =
                            u128::from(axis.step_quotient * axis.denominator + axis.step_remainder);
                        let numerator = initial + u128::from(offset) * step;
                        axis.quotient = (numerator / u128::from(axis.denominator)) as u64;
                        axis.remainder = (numerator % u128::from(axis.denominator)) as u64;
                        for coordinate in offset..output.min(offset.saturating_add(8)) {
                            assert_eq!(
                                axis.coordinate(),
                                map_axis_coordinate(coordinate, source, output, alignment),
                                "{source} -> {output}, {alignment:?}, x={coordinate}"
                            );
                            axis.advance();
                        }
                    }
                }
            }
        }
    }
}
