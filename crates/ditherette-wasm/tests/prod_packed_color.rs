use ditherette_wasm::{
    image::{ImageDimensions, ImageView, Rgba8, RowStride},
    prod::color::packed::{Converter, PackedSpace},
    spec::{self, contract::request::WorkingSpace},
};

#[test]
fn packed_image_matches_frozen_bits_for_every_space_stride_and_gray_endpoint() {
    let spaces = [
        (PackedSpace::Srgb, WorkingSpace::Srgb),
        (PackedSpace::LinearRgb, WorkingSpace::LinearRgb),
        (PackedSpace::Oklab, WorkingSpace::Oklab),
        (PackedSpace::Oklch, WorkingSpace::Oklch),
        (PackedSpace::Cielab, WorkingSpace::Cielab),
        (PackedSpace::Cielch, WorkingSpace::Cielch),
        (PackedSpace::Ycbcr, WorkingSpace::Ycbcr),
    ];
    let seam_colors = [[255, 0, 0], [255, 0, 1], [255, 1, 0]];
    for (width, height) in [(1, 1), (5, 7), (257, 3)] {
        let dimensions = ImageDimensions::new(width, height).unwrap();
        for padding in [0, 13] {
            let stride = width as usize * 4 + padding;
            let mut bytes = vec![203; stride * height as usize];
            for y in 0..height as usize {
                for x in 0..width as usize {
                    let index = y * width as usize + x;
                    let rgb = match index {
                        0..=255 => [index as u8; 3],
                        256..=258 => seam_colors[index - 256],
                        _ => [(index * 73) as u8, (index * 31) as u8, (index * 17) as u8],
                    };
                    let offset = y * stride + x * 4;
                    bytes[offset..offset + 3].copy_from_slice(&rgb);
                    bytes[offset + 3] = [0, 1, 127, 128, 254, 255][index % 6];
                }
            }
            let original = bytes.clone();
            let source =
                ImageView::<Rgba8>::new(&bytes, dimensions, RowStride::new(stride).unwrap())
                    .unwrap();
            for (space, reference) in spaces {
                let length = dimensions.pixel_count().unwrap() * 3;
                let mut output = vec![f32::NEG_INFINITY; length + 2];
                Converter::new(space).rgba8_into(source, &mut output[1..length + 1]);
                assert_eq!(output[0], f32::NEG_INFINITY);
                assert_eq!(output[length + 1], f32::NEG_INFINITY);
                for y in 0..height {
                    for x in 0..width {
                        let pixel = source.pixel(x, y).unwrap();
                        let expected = spec::color::rgb8_to_coordinates(
                            [pixel[0], pixel[1], pixel[2]],
                            reference,
                        );
                        let offset = 1 + (y as usize * width as usize + x as usize) * 3;
                        let actual = &output[offset..offset + 3];
                        assert_eq!(
                            actual.iter().copied().map(f32::to_bits).collect::<Vec<_>>(),
                            expected.map(f32::to_bits),
                            "{space:?}, {width}x{height}, padding {padding}, pixel {x},{y}"
                        );
                    }
                }
            }
            assert_eq!(bytes, original);
        }
    }
}
