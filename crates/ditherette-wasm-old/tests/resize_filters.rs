use ditherette_wasm_old::{image::ImageDimensions, resize::resize_rgba_area};

#[test]
fn area_identity_preserves_pixels() {
    let source_rgba: Vec<u8> = (0..4 * 4 * 4)
        .map(|value| (value * 37 % 251) as u8)
        .collect();
    let dimensions = dimensions(4, 4);

    assert_eq!(
        resize_rgba_area(&source_rgba, dimensions, dimensions).unwrap(),
        source_rgba
    );
}

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}
