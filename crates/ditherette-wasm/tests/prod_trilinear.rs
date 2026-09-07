use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Oklab32, Rgba8, RowStride},
    prod::resize::scalar::{
        bilinear::alignment::ResizeAnchor as ProdAnchor, trilinear::resize_trilinear_into,
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecAnchor,
        scalar::trilinear::resize_trilinear_into as oracle,
    },
};

const ANCHORS: [(ProdAnchor, SpecAnchor); 9] = [
    (ProdAnchor::TopLeft, SpecAnchor::TopLeft),
    (ProdAnchor::Top, SpecAnchor::Top),
    (ProdAnchor::TopRight, SpecAnchor::TopRight),
    (ProdAnchor::Left, SpecAnchor::Left),
    (ProdAnchor::Center, SpecAnchor::Center),
    (ProdAnchor::Right, SpecAnchor::Right),
    (ProdAnchor::BottomLeft, SpecAnchor::BottomLeft),
    (ProdAnchor::Bottom, SpecAnchor::Bottom),
    (ProdAnchor::BottomRight, SpecAnchor::BottomRight),
];

#[test]
fn exact_rgba_mips_lod_anchors_and_strided_rows() {
    for (sw, sh, ow, oh) in [
        (1, 1, 7, 9),
        (7, 9, 7, 9),
        (7, 9, 3, 4),
        (7, 9, 2, 3),
        (8, 8, 2, 2),
        (17, 13, 1, 1),
        (31, 1, 3, 1),
        (1, 31, 1, 3),
        (31, 3, 2, 17),
        (2, 17, 31, 3),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let source_stride = sw as usize * 4 + 3;
        let output_stride = ow as usize * 4 + 5;
        let mut bytes = vec![201; source_stride * sh as usize];
        for y in 0..sh as usize {
            for x in 0..sw as usize * 4 {
                bytes[y * source_stride + x] = if x % 4 == 3 {
                    [0, 1, 127, 254, 255][(x / 4 + y) % 5]
                } else {
                    ((x * 73 + y * 19) % 256) as u8
                };
            }
        }
        for (anchor, spec_anchor) in ANCHORS {
            let mut expected = vec![211; output_stride * oh as usize];
            let mut actual = expected.clone();
            oracle(
                ImageView::<Rgba8>::new(&bytes, source, RowStride::new(source_stride).unwrap())
                    .unwrap(),
                ImageViewMut::new(
                    &mut expected,
                    output,
                    RowStride::new(output_stride).unwrap(),
                )
                .unwrap(),
                spec_anchor,
            );
            resize_trilinear_into(
                ImageView::<Rgba8>::new(&bytes, source, RowStride::new(source_stride).unwrap())
                    .unwrap(),
                ImageViewMut::new(&mut actual, output, RowStride::new(output_stride).unwrap())
                    .unwrap(),
                anchor,
            );
            assert_eq!(actual, expected, "{sw}x{sh}->{ow}x{oh}, {anchor:?}");
        }
    }
}

#[test]
fn intermediate_float_storage_rounds_identically() {
    let source = ImageDimensions::new(7, 9).unwrap();
    let output = ImageDimensions::new(2, 3).unwrap();
    let bytes: Vec<f32> = (0..7 * 9 * 3).map(|x| x as f32 / 73.0 - 0.7).collect();
    for (anchor, spec_anchor) in ANCHORS {
        let mut expected = vec![0.0f32; 2 * 3 * 3];
        let mut actual = expected.clone();
        oracle(
            ImageView::<Oklab32>::packed(&bytes, source).unwrap(),
            ImageViewMut::packed(&mut expected, output).unwrap(),
            spec_anchor,
        );
        resize_trilinear_into(
            ImageView::<Oklab32>::packed(&bytes, source).unwrap(),
            ImageViewMut::packed(&mut actual, output).unwrap(),
            anchor,
        );
        assert_eq!(
            actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }
}
