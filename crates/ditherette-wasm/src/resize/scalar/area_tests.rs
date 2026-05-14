use super::resize_rgba_area_scalar_into;
use crate::{
    image::{rgba, ImageDimensions},
    resize::{area::resize_rgba_area, reference::area::resize_rgba_area_reference},
};

#[test]
fn four_by_four_to_two_by_two_averages_covered_pixels() {
    let source_rgba: Vec<u8> = (0..16)
        .flat_map(|value| [value, value, value, 255])
        .collect();

    let output_rgba = resize_rgba_area(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap();
    let reference_rgba =
        resize_rgba_area_reference(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap();

    let expected_rgba = [
        [3, 3, 3, 255],
        [5, 5, 5, 255],
        [11, 11, 11, 255],
        [13, 13, 13, 255],
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
    assert_eq!(reference_rgba, expected_rgba);
}

#[test]
fn scalar_matches_reference_across_resize_shapes() {
    for (source_dimensions, output_dimensions) in [
        (dimensions(4, 4), dimensions(2, 2)),
        (dimensions(5, 3), dimensions(4, 2)),
        (dimensions(3, 2), dimensions(5, 4)),
        (dimensions(4, 5), dimensions(4, 3)),
        (dimensions(4, 3), dimensions(4, 3)),
        (dimensions(1, 1), dimensions(5, 4)),
    ] {
        assert_scalar_matches_reference(source_dimensions, output_dimensions);
    }
}

fn assert_scalar_matches_reference(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) {
    let source_rgba = patterned_rgba(source_dimensions);
    let reference =
        resize_rgba_area_reference(&source_rgba, source_dimensions, output_dimensions).unwrap();
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions).unwrap();
    let mut scalar = vec![0xA5; output_byte_len];

    resize_rgba_area_scalar_into(
        &source_rgba,
        source_dimensions,
        output_dimensions,
        &mut scalar,
    )
    .unwrap();

    assert_eq!(scalar, reference);
}

fn patterned_rgba(dimensions: ImageDimensions) -> Vec<u8> {
    let pixel_count = dimensions.width() as usize * dimensions.height() as usize;

    (0..pixel_count)
        .flat_map(|index| {
            let value = index as u8;
            [value, value.wrapping_mul(3), value.wrapping_mul(5), 255]
        })
        .collect()
}

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}
