use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    spec::{adapters::*, color, contract::request::WorkingSpace, tiling::RowBand},
};

#[test]
fn legacy_color_aliases_and_byte_alpha_keep_the_four_channel_contract() {
    let cases = [
        (WorkingSpace::Srgb, ["srgb", "srgb32", "srgb-f32"]),
        (
            WorkingSpace::LinearRgb,
            ["linear-srgb", "linear-srgb-f32", "linear-rgba32"],
        ),
        (WorkingSpace::Oklab, ["oklab", "oklab-f32", "oklaba32"]),
        (WorkingSpace::Oklch, ["oklch", "oklch-f32", "oklch32"]),
        (WorkingSpace::Cielab, ["cielab", "cielab-f32", "cielab32"]),
        (WorkingSpace::Cielch, ["cielch", "cielch-f32", "cielch32"]),
        (WorkingSpace::Ycbcr, ["ycbcr", "ycbcr-f32", "ycbcr32"]),
    ];
    for (space, aliases) in cases {
        for alias in aliases {
            assert_eq!(legacy_color_space(alias), Some(space));
        }
    }
    for source_name in ["rgba8", "srgb-rgba8", "srgb"] {
        assert_eq!(
            legacy_convert_color_space(&[255, 0, 0, 128, 0, 255, 0, 0], 2, 1, source_name, "srgb")
                .unwrap(),
            vec![1.0, 0.0, 0.0, 128.0 / 255.0, 0.0, 1.0, 0.0, 0.0]
        );
    }
    assert_eq!(
        legacy_convert_color_space(&[], 0, 0, "bad", "bad").unwrap_err(),
        "unsupported source color space"
    );
    assert_eq!(
        legacy_convert_color_space(&[], 1, 1, "rgba8", "bad").unwrap_err(),
        "input length does not match RGBA8 dimensions"
    );
    assert_eq!(
        legacy_convert_color_space(&[0; 4], 1, 1, "rgba8", "bad").unwrap_err(),
        "unsupported target color space"
    );
    assert_eq!(
        legacy_hello("Mia"),
        "Hello, Mia, from Ditherette's fresh Rust core!"
    );
}

#[test]
fn legacy_cylindrical_gray_residuals_are_not_public_neutral_coordinates() {
    let gray = [8, 8, 8, 99];
    let legacy = legacy_convert_color_space(&gray, 1, 1, "rgba8", "cielch").unwrap();
    assert!((legacy[1] - 0.000_008_024_521).abs() < 1e-10);
    assert!((legacy[2] - 2.761_086_2).abs() < 1e-6);
    assert_eq!(&color::cielch::rgb8_to_cielch([8; 3])[1..], &[0.0, 0.0]);
    assert_eq!(legacy[3], 99.0 / 255.0);
}

#[test]
fn color_row_projection_ignores_padding_and_preserves_other_rows_in_every_space() {
    let bytes = [255, 0, 0, 17, 77, 77, 0, 255, 0, 0, 77, 77, 0, 0, 255, 255];
    let source = ImageView::<Rgba8>::new(
        &bytes,
        ImageDimensions::new(1, 3).unwrap(),
        RowStride::new(6).unwrap(),
    )
    .unwrap();
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        let mut whole = [0.0; 12];
        legacy_color_rows_into(source, space, RowBand::new(0, 3).unwrap(), &mut whole);
        let mut selected = [-123.0; 12];
        legacy_color_rows_into(source, space, RowBand::new(1, 2).unwrap(), &mut selected);
        assert_eq!(&selected[..4], &[-123.0; 4]);
        assert_eq!(&selected[4..8], &whole[4..8]);
        assert_eq!(&selected[8..], &[-123.0; 4]);
        for band in [RowBand::new(2, 3).unwrap(), RowBand::new(0, 1).unwrap()] {
            legacy_color_rows_into(source, space, band, &mut selected);
        }
        assert_eq!(selected, whole);
        assert_eq!([whole[3], whole[7], whole[11]], [17.0 / 255.0, 0.0, 1.0]);
    }
}

#[test]
fn legacy_process_is_resize_only_and_retains_crop_and_mode_validation() {
    let bytes = [0, 0, 0, 0, 100, 0, 0, 100, 200, 0, 0, 200, 255, 0, 0, 255];
    let settings = r#"{"output":{"width":1,"height":1,"resize":"area","crop":{"x":0,"y":0,"width":2,"height":2}},"palette":[],"ignored":true}"#;
    assert_eq!(
        legacy_process_rgba8(&bytes, 2, 2, settings, true).unwrap(),
        vec![139, 0, 0, 139]
    );
    assert_eq!(
        legacy_resize_rgba8(&bytes, 2, 2, 1, 1, "nearest", "center", "", false).unwrap(),
        vec![255, 0, 0, 255]
    );
    let cropped = r#"{"output":{"width":1,"height":1,"resize":"area","crop":{"x":1,"y":0,"width":1,"height":2}}}"#;
    assert_eq!(
        legacy_process_rgba8(&bytes, 2, 2, cropped, false).unwrap_err(),
        "processRgba8 does not support cropped sources yet"
    );
    for mode in [
        "nearest",
        "area",
        "bilinear",
        "bicubic",
        "bicubic-catmull-rom",
        "lanczos2",
        "lanczos3",
        "lanczos2-scale-aware",
        "lanczos3-scale-aware",
    ] {
        let settings = format!(r#"{{"output":{{"width":1,"height":1,"resize":"{mode}"}}}}"#);
        assert_eq!(
            legacy_process_rgba8(&[8, 9, 10, 11], 1, 1, &settings, false).unwrap(),
            vec![8, 9, 10, 11]
        );
    }
    assert!(legacy_process_rgba8(&bytes, 2, 2, "{}", false)
        .unwrap_err()
        .starts_with("invalid process settings:"));
    assert_eq!(
        legacy_process_rgba8(
            &bytes,
            2,
            2,
            r#"{"output":{"width":1,"height":1,"resize":"trilinear"}}"#,
            false
        )
        .unwrap_err(),
        "unsupported process resize mode"
    );
}

#[test]
fn row_and_plan_adapters_project_complete_resize_coordinates() {
    let source_bytes = [10, 0, 0, 20, 30, 0, 0, 40, 50, 0, 0, 60];
    let source = ImageView::packed(&source_bytes, ImageDimensions::new(1, 3).unwrap()).unwrap();
    let dimensions = ImageDimensions::new(1, 5).unwrap();
    let nearest = LegacyResize::parse("nearest", "center", "fixed").unwrap();
    let mut middle = [0; 12];
    nearest.resize_rows_into(source, dimensions, RowBand::new(1, 4).unwrap(), &mut middle);
    assert_eq!(middle, [10, 0, 0, 20, 30, 0, 0, 40, 50, 0, 0, 60]);
    for filter in [
        "nearest", "area", "bilinear", "bicubic", "lanczos2", "lanczos3",
    ] {
        let resize = LegacyResize::parse(filter, "bottom-right", "scale-aware").unwrap();
        let mut whole = [0; 20];
        resize.resize_into(
            source,
            ImageViewMut::packed(&mut whole, dimensions).unwrap(),
        );
        let mut split = [0; 20];
        resize.resize_rows_into(
            source,
            dimensions,
            RowBand::new(0, 2).unwrap(),
            &mut split[..8],
        );
        resize.resize_rows_into(
            source,
            dimensions,
            RowBand::new(2, 5).unwrap(),
            &mut split[8..],
        );
        assert_eq!(split, whole);
    }
    for support in ["fixed+pooled-direct", "fixed+pooled-direct+per-band-plan"] {
        let resize = LegacyResize::parse("lanczos2", "center", support).unwrap();
        assert_eq!(resize.execution, LegacyExecution::PooledDirect);
        assert_eq!(resize.per_band_plan, support.ends_with("per-band-plan"));
        let mut whole = [0; 20];
        resize.resize_into(
            source,
            ImageViewMut::packed(&mut whole, dimensions).unwrap(),
        );
        for height in [0, 1, 2, 3, 8] {
            let mut split = [0; 20];
            resize
                .pooled_direct_into(source, dimensions, height, &mut split)
                .unwrap();
            assert_eq!(split, whole);
        }
    }
    assert_eq!(
        LegacyResize::parse("area", "center", "fixed+pooled-direct")
            .unwrap()
            .pooled_direct_into(source, dimensions, 2, &mut [0; 20])
            .unwrap_err(),
        "resize filter does not support pooled direct"
    );
}

#[test]
fn diagnostics_preserve_storage_recipes_including_last_band_phase() {
    let bytes: Vec<u8> = (0..12).collect();
    let source = ImageView::packed(&bytes, ImageDimensions::new(1, 3).unwrap()).unwrap();
    let dimensions = ImageDimensions::new(1, 5).unwrap();
    let mut output = [0; 20];
    diagnostic_resize_copy_into(source, dimensions, 2, &mut output);
    assert_eq!(
        output,
        [0, 1, 2, 3, 4, 5, 6, 7, 4, 5, 6, 7, 8, 9, 10, 11, 4, 5, 6, 7]
    );
    diagnostic_resize_copy_into(source, dimensions, 1, &mut output);
    assert_eq!(
        output,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5, 6, 7]
    );
    let mut floats = [0.0; 12];
    diagnostic_color_copy_into(source, &mut floats);
    assert_eq!(
        floats,
        [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]
    );
    let old_output = output;
    diagnostic_noop(&mut output);
    assert_eq!(output, old_output);
    let mut special = [f32::from_bits(0x7fc0_1234), -0.0, f32::INFINITY];
    let old_bits = special.map(f32::to_bits);
    diagnostic_noop(&mut special);
    assert_eq!(special.map(f32::to_bits), old_bits);
}

#[test]
fn diagnostic_resize_modes_require_enabled_policy_and_keep_parse_order() {
    let bytes = [8, 9, 10, 11];
    assert_eq!(
        legacy_resize_rgba8(
            &bytes,
            1,
            1,
            1,
            1,
            "nearest",
            "center",
            "fixed+pooled-noop",
            true
        )
        .unwrap(),
        vec![0; 4]
    );
    assert_eq!(
        legacy_resize_rgba8(
            &bytes,
            1,
            1,
            1,
            1,
            "nearest",
            "center",
            "fixed+pooled-noop",
            false
        )
        .unwrap(),
        bytes
    );
    assert_eq!(
        LegacyResize::parse("bad", "bad", "bad").unwrap_err(),
        "unsupported resize filter"
    );
    assert_eq!(
        LegacyResize::parse("area", "bad", "bad").unwrap_err(),
        "unsupported support policy"
    );
    assert_eq!(
        LegacyResize::parse("area", "bad", "fixed").unwrap_err(),
        "unsupported resize anchor"
    );
}
