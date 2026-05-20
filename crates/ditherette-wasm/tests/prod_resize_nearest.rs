use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    prod::resize::scalar::nearest::{
        alignment::ResizeAnchor as ProdResizeAnchor,
        resize_nearest_into as resize_prod_nearest_into,
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecResizeAnchor,
        scalar::nearest::resize_nearest_into as resize_spec_nearest_into,
    },
};

#[test]
fn prod_nearest_matches_spec_for_anchor_matrix() {
    let source_dimensions = ImageDimensions::new(4, 3).unwrap();
    let output_dimensions = ImageDimensions::new(7, 5).unwrap();
    let source = numbered_rgba_source(source_dimensions);

    for (spec_anchor, prod_anchor) in anchors() {
        let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

        resize_spec_nearest_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
            spec_anchor,
        );
        resize_prod_nearest_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
            prod_anchor,
        );

        assert_eq!(prod_output, spec_output, "anchor {spec_anchor:?}");
    }
}

#[test]
fn prod_nearest_matches_spec_with_strided_rows() {
    let source_dimensions = ImageDimensions::new(3, 2).unwrap();
    let output_dimensions = ImageDimensions::new(5, 3).unwrap();
    let source_stride = RowStride::new(16).unwrap();
    let output_stride = RowStride::new(24).unwrap();
    let mut source = vec![99; source_stride.elements() * source_dimensions.height_usize()];
    for y in 0..source_dimensions.height_usize() {
        for x in 0..source_dimensions.width_usize() {
            let pixel = y * source_stride.elements() + x * 4;
            let value = (y * source_dimensions.width_usize() + x) as u8;
            source[pixel..pixel + 4].copy_from_slice(&[value, value.wrapping_add(1), 0, 255]);
        }
    }
    let mut spec_output = vec![0; output_stride.elements() * output_dimensions.height_usize()];
    let mut prod_output = vec![0; output_stride.elements() * output_dimensions.height_usize()];

    resize_spec_nearest_into(
        ImageView::<Rgba8>::new(&source, source_dimensions, source_stride).unwrap(),
        ImageViewMut::<Rgba8>::new(&mut spec_output, output_dimensions, output_stride).unwrap(),
        SpecResizeAnchor::Center,
    );
    resize_prod_nearest_into(
        ImageView::<Rgba8>::new(&source, source_dimensions, source_stride).unwrap(),
        ImageViewMut::<Rgba8>::new(&mut prod_output, output_dimensions, output_stride).unwrap(),
        ProdResizeAnchor::Center,
    );

    assert_eq!(prod_output, spec_output);
}

fn anchors() -> [(SpecResizeAnchor, ProdResizeAnchor); 9] {
    [
        (SpecResizeAnchor::TopLeft, ProdResizeAnchor::TopLeft),
        (SpecResizeAnchor::Top, ProdResizeAnchor::Top),
        (SpecResizeAnchor::TopRight, ProdResizeAnchor::TopRight),
        (SpecResizeAnchor::Left, ProdResizeAnchor::Left),
        (SpecResizeAnchor::Center, ProdResizeAnchor::Center),
        (SpecResizeAnchor::Right, ProdResizeAnchor::Right),
        (SpecResizeAnchor::BottomLeft, ProdResizeAnchor::BottomLeft),
        (SpecResizeAnchor::Bottom, ProdResizeAnchor::Bottom),
        (SpecResizeAnchor::BottomRight, ProdResizeAnchor::BottomRight),
    ]
}

fn numbered_rgba_source(dimensions: ImageDimensions) -> Vec<u8> {
    let mut source = Vec::with_capacity(dimensions.storage_len::<Rgba8>().unwrap());
    for index in 0..dimensions.pixel_count().unwrap() {
        source.extend_from_slice(&[index as u8, index.wrapping_mul(3) as u8, 0, 255]);
    }
    source
}
