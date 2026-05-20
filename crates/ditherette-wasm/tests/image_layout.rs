use ditherette_wasm::image::{
    ImageBuf, ImageDimensions, ImageLayoutError, ImageView, ImageViewMut, Oklab32, PaletteIndex8,
    Rgba8, RowStride,
};

#[test]
fn dimensions_reject_zero_axes_and_calculate_lengths() {
    assert_eq!(ImageDimensions::new(0, 1), Err(ImageLayoutError::ZeroWidth));
    assert_eq!(
        ImageDimensions::new(1, 0),
        Err(ImageLayoutError::ZeroHeight)
    );

    let dimensions = ImageDimensions::new(3, 2).unwrap();
    assert_eq!(dimensions.pixel_count().unwrap(), 6);
    assert_eq!(dimensions.storage_len::<Rgba8>().unwrap(), 24);
    assert_eq!(dimensions.storage_len::<Oklab32>().unwrap(), 18);
    assert_eq!(dimensions.byte_len::<Rgba8>().unwrap(), 24);
    assert_eq!(dimensions.byte_len::<Oklab32>().unwrap(), 72);
}

#[test]
fn format_markers_describe_flat_channel_layouts() {
    use ditherette_wasm::image::ImageFormat;

    assert_eq!(Rgba8::CHANNEL_COUNT, 4);
    assert_eq!([Rgba8::R, Rgba8::G, Rgba8::B, Rgba8::A], [0, 1, 2, 3]);
    assert_eq!(Oklab32::CHANNEL_COUNT, 3);
    assert_eq!([Oklab32::L, Oklab32::A, Oklab32::B], [0, 1, 2]);
    assert_eq!(PaletteIndex8::CHANNEL_COUNT, 1);
    assert_eq!(PaletteIndex8::INDEX, 0);
}

#[test]
fn row_stride_is_measured_in_storage_elements() {
    let dimensions = ImageDimensions::new(7, 3).unwrap();

    assert_eq!(
        RowStride::packed::<Rgba8>(dimensions).unwrap().elements(),
        28
    );
    assert_eq!(
        RowStride::packed::<Oklab32>(dimensions).unwrap().elements(),
        21
    );

    let too_small = RowStride::new(15).unwrap();
    assert_eq!(
        too_small.validate_for::<Rgba8>(ImageDimensions::new(4, 3).unwrap()),
        Err(ImageLayoutError::StrideTooSmall {
            stride: 15,
            row_len: 16,
        })
    );
}

#[test]
fn packed_views_require_exact_storage_length() {
    let dimensions = ImageDimensions::new(2, 2).unwrap();
    let data = [1_u8, 2, 3];

    assert_eq!(
        ImageView::<Rgba8>::packed(&data, dimensions).unwrap_err(),
        ImageLayoutError::BufferLengthMismatch {
            len: 3,
            expected: 16,
        }
    );
}

#[test]
fn strided_views_accept_padding_and_exclude_it_from_rows() {
    let dimensions = ImageDimensions::new(2, 2).unwrap();
    let stride = RowStride::new(10).unwrap();
    let data = [
        1_u8, 2, 3, 4, 5, 6, 7, 8, 99, 99, // row 0 + padding
        9, 10, 11, 12, 13, 14, 15, 16, // row 1, no trailing padding required
    ];
    let view = ImageView::<Rgba8>::new(&data, dimensions, stride).unwrap();

    assert_eq!(view.row(0).unwrap(), &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(view.row(1).unwrap(), &[9, 10, 11, 12, 13, 14, 15, 16]);
    assert_eq!(view.row(2), None);
}

#[test]
fn strided_views_reject_short_buffers() {
    let dimensions = ImageDimensions::new(4, 3).unwrap();
    let stride = RowStride::new(20).unwrap();
    let data = vec![0_u8; 55];

    assert_eq!(
        ImageView::<Rgba8>::new(&data, dimensions, stride).unwrap_err(),
        ImageLayoutError::BufferTooShort {
            len: 55,
            required: 56,
        }
    );
}

#[test]
fn views_read_and_write_flat_channels_without_pixel_structs() {
    let dimensions = ImageDimensions::new(2, 1).unwrap();
    let mut data = [0_u8; 8];
    let mut view = ImageViewMut::<Rgba8>::packed(&mut data, dimensions).unwrap();

    assert_eq!(view.set_channel(1, 0, Rgba8::A, 255), Some(()));
    assert_eq!(view.channel(1, 0, Rgba8::A), Some(255));
    assert_eq!(view.set_channel(1, 0, 4, 9), None);
    assert_eq!(view.pixel(1, 0).unwrap(), &[0, 0, 0, 255]);
}

#[test]
fn owned_buffers_create_and_expose_views() {
    let dimensions = ImageDimensions::new(3, 2).unwrap();
    let image = ImageBuf::<Rgba8>::new_packed(dimensions).unwrap();

    assert_eq!(image.data(), &[0; 24]);
    assert_eq!(image.stride().elements(), 12);

    let dimensions = ImageDimensions::new(2, 1).unwrap();
    let mut image =
        ImageBuf::<Rgba8>::from_vec_packed(vec![1_u8, 2, 3, 4, 5, 6, 7, 8], dimensions).unwrap();

    assert_eq!(image.as_view().channel(1, 0, Rgba8::G), Some(6));
    image
        .as_view_mut()
        .set_channel(0, 0, Rgba8::A, 255)
        .unwrap();
    assert_eq!(image.as_view().channel(0, 0, Rgba8::A), Some(255));
}

#[test]
fn owned_buffers_reject_wrong_packed_length_and_support_f32_formats() {
    let dimensions = ImageDimensions::new(2, 2).unwrap();

    assert_eq!(
        ImageBuf::<Rgba8>::from_vec_packed(vec![1_u8, 2, 3], dimensions).unwrap_err(),
        ImageLayoutError::BufferLengthMismatch {
            len: 3,
            expected: 16,
        }
    );

    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let image = ImageBuf::<Oklab32>::from_vec_packed(vec![0.1, 0.2, 0.3], dimensions).unwrap();
    assert_eq!(image.as_view().pixel(0, 0).unwrap(), &[0.1, 0.2, 0.3]);
}
