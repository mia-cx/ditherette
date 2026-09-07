//! Exact S21 copied-baseline conformance. Existing bounded tests cover legacy candidates only.

use ditherette_wasm::{
    image::{
        ImageDimensions, ImageFormat, ImageView, ImageViewMut, Oklab32, PaletteIndex8, Rgba8,
        RowStride,
    },
    prod::resize::{
        common::alignment::ResizeAnchor as ProdAnchor,
        scalar::{area, bilinear},
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecAnchor,
        scalar::{area as oracle_area, bilinear as oracle_bilinear},
    },
};

const ANCHORS: [(SpecAnchor, ProdAnchor); 9] = [
    (SpecAnchor::TopLeft, ProdAnchor::TopLeft),
    (SpecAnchor::Top, ProdAnchor::Top),
    (SpecAnchor::TopRight, ProdAnchor::TopRight),
    (SpecAnchor::Left, ProdAnchor::Left),
    (SpecAnchor::Center, ProdAnchor::Center),
    (SpecAnchor::Right, ProdAnchor::Right),
    (SpecAnchor::BottomLeft, ProdAnchor::BottomLeft),
    (SpecAnchor::Bottom, ProdAnchor::Bottom),
    (SpecAnchor::BottomRight, ProdAnchor::BottomRight),
];
const CASES: [((u32, u32), (u32, u32)); 11] = [
    ((1, 1), (9, 7)),
    ((7, 5), (7, 5)),
    ((7, 5), (3, 2)),
    ((3, 2), (7, 5)),
    ((9, 1), (1, 9)),
    ((1, 9), (9, 1)),
    ((31, 3), (2, 17)),
    ((2, 17), (31, 3)),
    ((257, 1), (1, 1)),
    ((1, 1), (257, 1)),
    ((1, 257), (1, 2)),
];

#[test]
fn rgba_and_index_bytes_match_frozen_shapes_anchors_and_independent_strides() {
    compare_bytes::<Rgba8>();
    compare_bytes::<PaletteIndex8>();
}

fn compare_bytes<F: ImageFormat<Storage = u8> + Copy>() {
    for ((sw, sh), (ow, oh)) in CASES {
        let sd = ImageDimensions::new(sw, sh).unwrap();
        let od = ImageDimensions::new(ow, oh).unwrap();
        for sp in [0, 3] {
            for op in [0, 5] {
                let ss = sw as usize * F::CHANNEL_COUNT + sp;
                let os = ow as usize * F::CHANNEL_COUNT + op;
                let mut source: Vec<u8> = (0..ss * sh as usize)
                    .map(|i| (i.wrapping_mul(73).wrapping_add(19) % 256) as u8)
                    .collect();
                if F::CHANNEL_COUNT == 4 {
                    for y in 0..sh as usize {
                        for x in 0..sw as usize {
                            source[y * ss + x * 4 + 3] = [0, 1, 127, 254, 255][(x + y) % 5];
                        }
                    }
                }
                let original = source.clone();
                let view = ImageView::<F>::new(&source, sd, RowStride::new(ss).unwrap()).unwrap();
                let mut expected = vec![203; os * oh as usize];
                let mut actual = expected.clone();
                oracle_area::resize_area_into(
                    view,
                    ImageViewMut::new(&mut expected, od, RowStride::new(os).unwrap()).unwrap(),
                );
                area::resize_area_into(
                    view,
                    ImageViewMut::new(&mut actual, od, RowStride::new(os).unwrap()).unwrap(),
                );
                assert_eq!(actual, expected, "area {sw}x{sh}->{ow}x{oh}");
                for (sa, pa) in ANCHORS {
                    expected.fill(203);
                    actual.fill(203);
                    oracle_bilinear::resize_bilinear_into(
                        view,
                        ImageViewMut::new(&mut expected, od, RowStride::new(os).unwrap()).unwrap(),
                        sa,
                    );
                    bilinear::resize_bilinear_into(
                        view,
                        ImageViewMut::new(&mut actual, od, RowStride::new(os).unwrap()).unwrap(),
                        pa,
                    );
                    assert_eq!(actual, expected, "bilinear {sw}x{sh}->{ow}x{oh} {pa:?}");
                    for row in actual.chunks_exact(os) {
                        assert!(row[ow as usize * F::CHANNEL_COUNT..]
                            .iter()
                            .all(|&byte| byte == 203));
                    }
                }
                assert_eq!(source, original);
            }
        }
    }
}

#[test]
fn packed_float_coordinates_match_exact_bits_including_strided_padding() {
    for ((sw, sh), (ow, oh)) in CASES {
        let sd = ImageDimensions::new(sw, sh).unwrap();
        let od = ImageDimensions::new(ow, oh).unwrap();
        let ss = sw as usize * 3 + 2;
        let os = ow as usize * 3 + 4;
        let source: Vec<f32> = (0..ss * sh as usize)
            .map(|i| (i as f32 * 0.03125 - 2.0).sin())
            .collect();
        let original: Vec<_> = source.iter().map(|v| v.to_bits()).collect();
        let view = ImageView::<Oklab32>::new(&source, sd, RowStride::new(ss).unwrap()).unwrap();
        let mut expected = vec![-17.25; os * oh as usize];
        let mut actual = expected.clone();
        oracle_area::resize_area_into(
            view,
            ImageViewMut::new(&mut expected, od, RowStride::new(os).unwrap()).unwrap(),
        );
        area::resize_area_into(
            view,
            ImageViewMut::new(&mut actual, od, RowStride::new(os).unwrap()).unwrap(),
        );
        assert!(actual
            .iter()
            .zip(&expected)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        for (sa, pa) in ANCHORS {
            expected.fill(-17.25);
            actual.fill(-17.25);
            oracle_bilinear::resize_bilinear_into(
                view,
                ImageViewMut::new(&mut expected, od, RowStride::new(os).unwrap()).unwrap(),
                sa,
            );
            bilinear::resize_bilinear_into(
                view,
                ImageViewMut::new(&mut actual, od, RowStride::new(os).unwrap()).unwrap(),
                pa,
            );
            assert!(
                actual
                    .iter()
                    .zip(&expected)
                    .all(|(a, b)| a.to_bits() == b.to_bits()),
                "{sw}x{sh}->{ow}x{oh} {pa:?}"
            );
            for row in actual.chunks_exact(os) {
                assert!(row[ow as usize * 3..].iter().all(|&v| v == -17.25));
            }
        }
        assert_eq!(
            source.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            original
        );
    }
}

#[test]
fn independent_average_vectors_include_hidden_rgb_and_alpha_rounding() {
    let sd = ImageDimensions::new(2, 1).unwrap();
    let od = ImageDimensions::new(1, 1).unwrap();
    let source = [200, 0, 100, 0, 0, 100, 200, 255];
    let view = ImageView::<Rgba8>::packed(&source, sd).unwrap();
    let mut output = [0; 4];
    area::resize_area_into(view, ImageViewMut::packed(&mut output, od).unwrap());
    assert_eq!(output, [100, 50, 150, 128]);
    bilinear::resize_bilinear_into(
        view,
        ImageViewMut::packed(&mut output, od).unwrap(),
        ProdAnchor::Center,
    );
    assert_eq!(output, [100, 50, 150, 128]);
    let sd = ImageDimensions::new(3, 1).unwrap();
    let od = ImageDimensions::new(2, 1).unwrap();
    let source = [0, 120, 240];
    let view = ImageView::<PaletteIndex8>::packed(&source, sd).unwrap();
    let mut output = [0; 2];
    area::resize_area_into(view, ImageViewMut::packed(&mut output, od).unwrap());
    assert_eq!(output, [40, 200]);
    bilinear::resize_bilinear_into(
        view,
        ImageViewMut::packed(&mut output, od).unwrap(),
        ProdAnchor::Center,
    );
    assert_eq!(output, [40, 200]);
}
