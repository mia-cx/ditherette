use ditherette_wasm::{convert_color_space, resize_rgba8};

#[test]
fn color_space_export_materializes_f32_buffer() {
    let output = convert_color_space(&[255, 0, 0, 128], 1, 1, "rgba8", "oklab-f32", true)
        .expect("color conversion should succeed");

    assert_eq!(output.len(), 4);
    assert!((output[0] - 0.627_955).abs() <= 0.000_001);
    assert!((output[3] - 128.0 / 255.0).abs() <= f32::EPSILON);
}

#[test]
fn resize_export_runs_prod_nearest() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    let output = resize_rgba8(&source, 2, 1, 4, 2, "nearest", "center", "fixed", true)
        .expect("resize should succeed");

    assert_eq!(output.len(), 4 * 2 * 4);
    assert_eq!(&output[0..4], &[255, 0, 0, 255]);
    assert_eq!(&output[12..16], &[0, 255, 0, 255]);
}
