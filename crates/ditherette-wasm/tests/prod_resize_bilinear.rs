use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::scalar::bilinear::{
        alignment::ResizeAnchor as ProdResizeAnchor,
        resize_bilinear_rgba8_into as resize_prod_bilinear_into,
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecResizeAnchor,
        scalar::bilinear::resize_bilinear_into as resize_spec_bilinear_into,
    },
};

#[test]
fn prod_bilinear_matches_spec_for_anchor_matrix() {
    for (source_dimensions, output_dimensions) in [
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(7, 5).unwrap(),
        ),
        (
            ImageDimensions::new(7, 5).unwrap(),
            ImageDimensions::new(4, 3).unwrap(),
        ),
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(8, 9).unwrap(),
        ),
    ] {
        assert_prod_matches_spec(source_dimensions, output_dimensions);
    }
}

fn assert_prod_matches_spec(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) {
    let source = numbered_rgba_source(source_dimensions);

    for (spec_anchor, prod_anchor) in anchors() {
        let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

        resize_spec_bilinear_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
            spec_anchor,
        );
        resize_prod_bilinear_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
            prod_anchor,
        );

        assert_eq!(prod_output, spec_output, "anchor {spec_anchor:?}");
    }
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
        source.extend_from_slice(&[
            index as u8,
            index.wrapping_mul(3) as u8,
            index.wrapping_mul(7) as u8,
            255,
        ]);
    }
    source
}
