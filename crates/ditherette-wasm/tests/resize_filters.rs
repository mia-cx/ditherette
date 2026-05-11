use ditherette_wasm::{
    image::ImageDimensions,
    resize::{
        antialias_rgba_box3, resize_rgba_area, resize_rgba_bicubic, resize_rgba_box,
        resize_rgba_lanczos2, resize_rgba_lanczos2_scale_aware, resize_rgba_lanczos3,
        resize_rgba_lanczos3_scale_aware, resize_rgba_trilinear,
    },
};

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}

#[test]
fn area_and_box_are_equivalent() {
    let source_rgba: Vec<u8> = (0..16)
        .flat_map(|value| [value, value, value, 255])
        .collect();

    assert_eq!(
        resize_rgba_area(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap(),
        resize_rgba_box(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap()
    );
}

#[test]
fn convolution_filters_preserve_identity() {
    let source_rgba: Vec<u8> = (0..9)
        .flat_map(|value| [value * 11, value * 7, value * 3, 255])
        .collect();
    let dimensions = dimensions(3, 3);

    for output_rgba in [
        resize_rgba_bicubic(&source_rgba, dimensions, dimensions).unwrap(),
        resize_rgba_lanczos2(&source_rgba, dimensions, dimensions).unwrap(),
        resize_rgba_lanczos3(&source_rgba, dimensions, dimensions).unwrap(),
        resize_rgba_lanczos2_scale_aware(&source_rgba, dimensions, dimensions).unwrap(),
        resize_rgba_lanczos3_scale_aware(&source_rgba, dimensions, dimensions).unwrap(),
        resize_rgba_trilinear(&source_rgba, dimensions, dimensions).unwrap(),
    ] {
        assert_eq!(output_rgba, source_rgba);
    }
}

#[test]
fn post_resize_antialias_blurs_local_neighbors() {
    let source_rgba: Vec<u8> = (0..9)
        .flat_map(|value| [value * 10, value * 10, value * 10, 255])
        .collect();

    let output_rgba = antialias_rgba_box3(&source_rgba, dimensions(3, 3)).unwrap();

    assert_eq!(&output_rgba[16..20], &[40, 40, 40, 255]);
}
