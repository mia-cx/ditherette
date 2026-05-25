use ditherette_wasm::{benchmark_resize_rgba8, convert_color_space, process_rgba8, resize_rgba8};

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

#[test]
fn process_export_runs_scalar_pipeline_shell() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    let settings = r#"{
        "output": {
            "width": 4,
            "height": 2,
            "resize": "nearest"
        },
        "colorSpace": "srgb",
        "dither": { "algorithm": "none" }
    }"#;
    let output = process_rgba8(&source, 2, 1, settings, true)
        .expect("process shell should run scalar resize");

    assert_eq!(output.len(), 4 * 2 * 4);
    assert_eq!(&output[0..4], &[255, 0, 0, 255]);
    assert_eq!(&output[12..16], &[0, 255, 0, 255]);
}

#[test]
fn benchmark_export_returns_samples_json() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    let output = benchmark_resize_rgba8(
        &source, 2, 1, 4, 2, "nearest", "center", "fixed", true, 2, 1.0, 0.01, 1, 0.01, false, None,
    )
    .expect("benchmark should succeed");

    assert!(output.contains("\"batchSize\":"));
    assert!(output.contains("\"totalIterations\":"));
    assert!(output.contains("\"samplesNs\":"));
}
