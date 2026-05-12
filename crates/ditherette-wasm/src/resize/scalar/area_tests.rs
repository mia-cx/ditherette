use crate::{
    image::ImageDimensions,
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

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}
