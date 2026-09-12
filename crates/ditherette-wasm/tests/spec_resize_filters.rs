use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    spec::resize::{
        common::alignment::ResizeAnchor,
        scalar::{
            area::resize_area_into,
            bicubic::resize_bicubic_into,
            bilinear::resize_bilinear_into,
            convolution::SupportPolicy,
            lanczos::{resize_lanczos2_into, resize_lanczos3_into},
            trilinear::resize_trilinear_into,
        },
    },
};

#[test]
fn area_averages_exact_source_coverage() {
    let source_dimensions = ImageDimensions::new(2, 2).unwrap();
    let output_dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [
        10, 0, 0, 255, 20, 0, 0, 255, // row 0
        30, 0, 0, 255, 40, 0, 0, 255, // row 1
    ];
    let mut output = [0; 4];

    resize_area_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut output, output_dimensions).unwrap(),
    );

    assert_eq!(output, [25, 0, 0, 255]);
}

#[test]
fn bilinear_center_samples_between_two_source_pixels() {
    let source_dimensions = ImageDimensions::new(2, 1).unwrap();
    let output_dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [10, 0, 0, 255, 20, 0, 0, 255];
    let mut output = [0; 4];

    resize_bilinear_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut output, output_dimensions).unwrap(),
        ResizeAnchor::Center,
    );

    assert_eq!(output, [15, 0, 0, 255]);
}

#[test]
fn bicubic_identity_preserves_pixels() {
    let dimensions = ImageDimensions::new(3, 1).unwrap();
    let source = [10, 1, 2, 255, 20, 3, 4, 255, 30, 5, 6, 255];
    let mut output = [0; 12];

    resize_bicubic_into(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut output, dimensions).unwrap(),
        ResizeAnchor::Center,
        SupportPolicy::Fixed,
    );

    assert_eq!(output, source);
}

#[test]
fn lanczos_identity_preserves_pixels() {
    let dimensions = ImageDimensions::new(3, 1).unwrap();
    let source = [10, 1, 2, 255, 20, 3, 4, 255, 30, 5, 6, 255];
    let mut lanczos2 = [0; 12];
    let mut lanczos3 = [0; 12];

    resize_lanczos2_into(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut lanczos2, dimensions).unwrap(),
        ResizeAnchor::Center,
        SupportPolicy::Fixed,
    );
    resize_lanczos3_into(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut lanczos3, dimensions).unwrap(),
        ResizeAnchor::Center,
        SupportPolicy::Fixed,
    );

    assert_eq!(lanczos2, source);
    assert_eq!(lanczos3, source);
}

#[test]
fn scale_aware_lanczos_preserves_constant_images() {
    let source_dimensions = ImageDimensions::new(4, 4).unwrap();
    let output_dimensions = ImageDimensions::new(1, 1).unwrap();
    let mut source = Vec::new();
    for _ in 0..source_dimensions.pixel_count().unwrap() {
        source.extend_from_slice(&[50, 80, 120, 255]);
    }
    let mut output = [0; 4];

    resize_lanczos3_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut output, output_dimensions).unwrap(),
        ResizeAnchor::Center,
        SupportPolicy::ScaleAware,
    );

    assert_eq!(output, [50, 80, 120, 255]);
}

#[test]
fn trilinear_preserves_constant_images_through_mip_policy() {
    let source_dimensions = ImageDimensions::new(4, 4).unwrap();
    let output_dimensions = ImageDimensions::new(1, 1).unwrap();
    let mut source = Vec::new();
    for _ in 0..source_dimensions.pixel_count().unwrap() {
        source.extend_from_slice(&[50, 80, 120, 255]);
    }
    let mut output = [0; 4];

    resize_trilinear_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut output, output_dimensions).unwrap(),
        ResizeAnchor::Center,
    );

    assert_eq!(output, [50, 80, 120, 255]);
}
