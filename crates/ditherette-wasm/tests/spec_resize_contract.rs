use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    spec::resize::{common::alignment::ResizeAnchor, scalar::trilinear::resize_trilinear_into},
};

#[test]
fn trilinear_reads_logical_rows_and_preserves_output_padding() {
    let source_dimensions = ImageDimensions::new(4, 2).unwrap();
    let output_dimensions = ImageDimensions::new(2, 1).unwrap();
    let source = [
        0, 0, 0, 255, 20, 0, 0, 255, 40, 0, 0, 255, 60, 0, 0, 255, 199, 199, 199, 199, 80, 0, 0,
        255, 100, 0, 0, 255, 120, 0, 0, 255, 140, 0, 0, 255,
    ];
    let mut output = [77; 12];
    resize_trilinear_into(
        ImageView::<Rgba8>::new(&source, source_dimensions, RowStride::new(20).unwrap()).unwrap(),
        ImageViewMut::<Rgba8>::new(&mut output, output_dimensions, RowStride::new(12).unwrap())
            .unwrap(),
        ResizeAnchor::Center,
    );
    // The two 2x2 footprints average to 50 and 90. Padding is not image data.
    assert_eq!(output, [50, 0, 0, 255, 90, 0, 0, 255, 77, 77, 77, 77]);
}
