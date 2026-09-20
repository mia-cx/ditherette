use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    prod::{
        contract::request::{Placement, WorkingSpace},
        dither::{blue_noise, perturb::perturb_by_field_rows_into},
        tiling::RowBand,
    },
    spec,
};
use sha2::{Digest, Sha256};

#[test]
fn frozen_rank_asset_and_literal_lookup_preserve_exact_bits() {
    assert_eq!(
        include_bytes!("../src/prod/dither/blue_noise/tile.rs"),
        include_bytes!("../src/spec/dither/blue_noise/tile.rs")
    );
    let mut digest = Sha256::new();
    for rank in blue_noise::BLUE_NOISE_32X32 {
        digest.update(rank.to_le_bytes());
    }
    assert_eq!(
        format!("{:x}", digest.finalize()),
        "bcd93746b99ef8ad678ad425f21e1890b4248050b1ea1b382800d7da977e5943"
    );
    for y in (0..97).chain([u32::MAX - 1, u32::MAX]) {
        for x in (0..97).chain([u32::MAX - 1, u32::MAX]) {
            assert_eq!(
                blue_noise::blue_noise_at(x, y).to_bits(),
                spec::dither::blue_noise::blue_noise_at(x, y).to_bits()
            );
        }
    }
}

#[test]
fn blue_noise_global_bands_match_frozen_reconstruction_alpha_and_padding() {
    for (width, height) in [(1, 2), (31, 3), (32, 3), (33, 35), (65, 3)] {
        let dimensions = ImageDimensions::new(width, height).unwrap();
        let stride = RowStride::new(width as usize * 4 + 8).unwrap();
        let mut bytes = vec![203; stride.elements() * height as usize];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let i = y * width as usize + x;
                bytes[y * stride.elements() + x * 4..y * stride.elements() + x * 4 + 4]
                    .copy_from_slice(&[
                        (i * 73) as u8,
                        (i * 31 + 127) as u8,
                        (i * 17 + 255) as u8,
                        [0, 1, 127, 128, 254, 255][i % 6],
                    ]);
            }
        }
        let source = ImageView::<Rgba8>::new(&bytes, dimensions, stride).unwrap();
        for space in [
            WorkingSpace::Srgb,
            WorkingSpace::LinearRgb,
            WorkingSpace::Oklab,
            WorkingSpace::Oklch,
            WorkingSpace::Cielab,
            WorkingSpace::Cielch,
            WorkingSpace::Ycbcr,
        ] {
            for strength in [0.0, 0.7, f32::MAX] {
                for placement in [
                    Placement::Everywhere {},
                    Placement::Adaptive {
                        radius: 2,
                        threshold: 10.0,
                        softness: 5.0,
                    },
                ] {
                    let mut actual = vec![211; bytes.len()];
                    let mut expected = actual.clone();
                    for y in (0..height).rev() {
                        perturb_by_field_rows_into(
                            source,
                            ImageViewMut::new(&mut actual, dimensions, stride).unwrap(),
                            space,
                            strength,
                            placement,
                            RowBand::new(y, y + 1).unwrap(),
                            |x, y, _| blue_noise::blue_noise_at(x, y),
                        );
                    }
                    spec::dither::perturb::perturb_by_field_rows_into(
                        source,
                        ImageViewMut::new(&mut expected, dimensions, stride).unwrap(),
                        serde_json::from_value(serde_json::to_value(space).unwrap()).unwrap(),
                        strength,
                        serde_json::from_value(serde_json::to_value(placement).unwrap()).unwrap(),
                        spec::tiling::contract::RowBand::new(0, height).unwrap(),
                        |x, y, _| spec::dither::blue_noise::blue_noise_at(x, y),
                    );
                    assert_eq!(
                        actual, expected,
                        "{width}/{space:?}/{strength}/{placement:?}"
                    );
                    for y in 0..height as usize {
                        let offset = y * stride.elements();
                        assert_eq!(
                            &actual[offset + width as usize * 4..offset + stride.elements()],
                            &[211; 8]
                        );
                        for x in 0..width as usize {
                            assert_eq!(actual[offset + x * 4 + 3], bytes[offset + x * 4 + 3]);
                            if strength == 0.0 {
                                assert_eq!(
                                    &actual[offset + x * 4..offset + x * 4 + 4],
                                    &bytes[offset + x * 4..offset + x * 4 + 4]
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
