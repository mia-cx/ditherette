use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::scalar::area::resize_area_rgba8_into as resize_prod_area_into,
    spec::resize::scalar::area::resize_area_into as resize_spec_area_into,
};

#[test]
fn prod_area_matches_spec_for_representative_rgba8_resizes() {
    let source_dimensions = ImageDimensions::new(5, 4).unwrap();
    let source = patterned_rgba_source(source_dimensions);

    for output_dimensions in [
        ImageDimensions::new(5, 4).unwrap(),
        ImageDimensions::new(2, 2).unwrap(),
        ImageDimensions::new(3, 7).unwrap(),
        ImageDimensions::new(8, 6).unwrap(),
        ImageDimensions::new(1, 3).unwrap(),
    ] {
        let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

        resize_spec_area_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
        );
        resize_prod_area_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
        );

        assert_eq!(prod_output, spec_output, "output {output_dimensions:?}");
    }
}

fn patterned_rgba_source(dimensions: ImageDimensions) -> Vec<u8> {
    let mut source = Vec::with_capacity(dimensions.storage_len::<Rgba8>().unwrap());
    for y in 0..dimensions.height_usize() {
        for x in 0..dimensions.width_usize() {
            source.extend_from_slice(&[
                (x * 31 + y * 17) as u8,
                (x * 13 + y * 43) as u8,
                (x * 7 + y * 19) as u8,
                255,
            ]);
        }
    }
    source
}
