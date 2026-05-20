use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Oklab32, Rgba8},
    spec::resize::{
        common::alignment::{map_axis_coordinate, AxisAlignment, ResizeAnchor},
        scalar::nearest::resize_nearest_into,
    },
};

#[test]
fn center_anchor_matches_old_nearest_downscale_mapping() {
    assert_eq!(mapping::<2>(4, 2, AxisAlignment::Center), [1, 3]);
    assert_eq!(mapping::<3>(4, 3, AxisAlignment::Center), [0, 2, 3]);
    assert_eq!(mapping::<4>(2, 4, AxisAlignment::Center), [0, 0, 1, 1]);
}

#[test]
fn start_center_and_end_alignment_are_distinct_when_ratio_exposes_them() {
    assert_eq!(mapping::<3>(4, 3, AxisAlignment::Start), [0, 1, 2]);
    assert_eq!(mapping::<3>(4, 3, AxisAlignment::Center), [0, 2, 3]);
    assert_eq!(mapping::<3>(4, 3, AxisAlignment::End), [1, 2, 3]);
}

#[test]
fn center_resizes_packed_rgba_without_destructuring_pixels() {
    let source_dimensions = ImageDimensions::new(2, 2).unwrap();
    let output_dimensions = ImageDimensions::new(4, 4).unwrap();
    let source = [
        1, 2, 3, 4, 5, 6, 7, 8, // row 0
        9, 10, 11, 12, 13, 14, 15, 16, // row 1
    ];
    let mut output = [0; 64];

    resize_nearest_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut output, output_dimensions).unwrap(),
        ResizeAnchor::Center,
    );

    assert_eq!(&output[0..4], &[1, 2, 3, 4]);
    assert_eq!(&output[4..8], &[1, 2, 3, 4]);
    assert_eq!(&output[8..12], &[5, 6, 7, 8]);
    assert_eq!(&output[60..64], &[13, 14, 15, 16]);
}

#[test]
fn top_left_and_bottom_right_choose_different_anchor_samples() {
    let source_dimensions = ImageDimensions::new(4, 4).unwrap();
    let output_dimensions = ImageDimensions::new(3, 3).unwrap();
    let source = numbered_rgba_source(source_dimensions);
    let mut top_left = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let mut bottom_right = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

    resize_nearest_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut top_left, output_dimensions).unwrap(),
        ResizeAnchor::TopLeft,
    );
    resize_nearest_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut bottom_right, output_dimensions).unwrap(),
        ResizeAnchor::BottomRight,
    );

    assert_eq!(&top_left[0..4], &[0, 0, 0, 255]);
    assert_eq!(&bottom_right[0..4], &[5, 0, 0, 255]);
    assert_eq!(&top_left[32..36], &[10, 0, 0, 255]);
    assert_eq!(&bottom_right[32..36], &[15, 0, 0, 255]);
}

#[test]
fn nearest_is_generic_over_f32_formats() {
    let source_dimensions = ImageDimensions::new(2, 1).unwrap();
    let output_dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [0.1, 0.2, 0.3, 0.7, 0.8, 0.9];
    let mut output = [0.0; 3];

    resize_nearest_into(
        ImageView::<Oklab32>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Oklab32>::packed(&mut output, output_dimensions).unwrap(),
        ResizeAnchor::Center,
    );

    assert_eq!(output, [0.7, 0.8, 0.9]);
}

#[test]
fn nearest_respects_strided_source_and_output_rows() {
    let source_dimensions = ImageDimensions::new(2, 2).unwrap();
    let output_dimensions = ImageDimensions::new(2, 2).unwrap();
    let source = [
        1, 2, 3, 4, 5, 6, 7, 8, 99, 99, // row 0 + padding
        9, 10, 11, 12, 13, 14, 15, 16,
    ];
    let mut output = [0; 20];

    resize_nearest_into(
        ImageView::<Rgba8>::new(
            &source,
            source_dimensions,
            ditherette_wasm::image::RowStride::new(10).unwrap(),
        )
        .unwrap(),
        ImageViewMut::<Rgba8>::new(
            &mut output,
            output_dimensions,
            ditherette_wasm::image::RowStride::new(10).unwrap(),
        )
        .unwrap(),
        ResizeAnchor::Center,
    );

    assert_eq!(&output[0..10], &[1, 2, 3, 4, 5, 6, 7, 8, 0, 0]);
    assert_eq!(&output[10..18], &[9, 10, 11, 12, 13, 14, 15, 16]);
}

fn mapping<const N: usize>(source_len: u32, output_len: u32, alignment: AxisAlignment) -> [u32; N] {
    std::array::from_fn(|index| {
        map_axis_coordinate(index as u32, source_len, output_len, alignment)
    })
}

fn numbered_rgba_source(dimensions: ImageDimensions) -> Vec<u8> {
    let mut source = Vec::with_capacity(dimensions.storage_len::<Rgba8>().unwrap());
    for index in 0..dimensions.pixel_count().unwrap() {
        source.extend_from_slice(&[index as u8, 0, 0, 255]);
    }
    source
}
