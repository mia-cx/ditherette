use ditherette_wasm::{
    benchmark_color_space, benchmark_resize_rgba8, convert_color_space, process_rgba8, resize_rgba8,
};

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
fn resize_export_runs_pooled_direct_nearest() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    let scalar = resize_rgba8(&source, 2, 1, 4, 2, "nearest", "center", "fixed", false)
        .expect("scalar resize should succeed");
    let pooled = resize_rgba8(
        &source,
        2,
        1,
        4,
        2,
        "nearest",
        "center",
        "fixed+pooled-direct",
        true,
    )
    .expect("pooled resize should succeed");

    assert_eq!(pooled, scalar);
}

#[test]
fn resize_export_runs_pooled_direct_lanczos() {
    let source = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let scalar = resize_rgba8(&source, 2, 2, 3, 3, "lanczos3", "center", "fixed", false)
        .expect("scalar resize should succeed");
    let pooled = resize_rgba8(
        &source,
        2,
        2,
        3,
        3,
        "lanczos3",
        "center",
        "fixed+pooled-direct",
        true,
    )
    .expect("pooled resize should succeed");

    assert_eq!(pooled, scalar);
}

#[test]
fn resize_export_runs_per_band_plan_lanczos() {
    let source = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let scalar = resize_rgba8(&source, 2, 2, 3, 3, "lanczos3", "center", "fixed", false)
        .expect("scalar resize should succeed");
    let per_band = resize_rgba8(
        &source,
        2,
        2,
        3,
        3,
        "lanczos3",
        "center",
        "fixed+pooled-direct+per-band-plan",
        true,
    )
    .expect("per-band plan resize should succeed");

    assert_eq!(per_band, scalar);
}

#[test]
fn resize_export_accepts_diagnostic_modes() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    for support_policy in ["fixed+pooled-noop", "fixed+pooled-copy"] {
        let output = resize_rgba8(
            &source,
            2,
            1,
            4,
            2,
            "nearest",
            "center",
            support_policy,
            true,
        )
        .expect("diagnostic resize should succeed");
        assert_eq!(output.len(), 4 * 2 * 4);
    }
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
fn color_benchmark_export_returns_samples_json() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    let output = benchmark_color_space(
        &source,
        2,
        1,
        "oklab-f32",
        "scalar",
        2,
        1.0,
        0.01,
        1,
        0.01,
        false,
        32,
        None,
    )
    .expect("color benchmark should succeed");

    assert!(output.contains("\"batchSize\":"));
    assert!(output.contains("\"totalIterations\":"));
    assert!(output.contains("\"samplesNs\":"));
}

#[test]
fn color_benchmark_accepts_diagnostic_modes() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    for mode in ["pooled_noop", "pooled_copy"] {
        let output = benchmark_color_space(
            &source,
            2,
            1,
            "oklab-f32",
            mode,
            1,
            0.01,
            0.01,
            1,
            0.01,
            false,
            32,
            None,
        )
        .expect("diagnostic color benchmark should succeed");
        assert!(output.contains("\"samplesNs\":"));
    }
}

#[test]
fn benchmark_export_returns_samples_json() {
    let source = [255, 0, 0, 255, 0, 255, 0, 255];
    let output = benchmark_resize_rgba8(
        &source, 2, 1, 4, 2, "nearest", "center", "fixed", true, 2, 1.0, 0.01, 1, 0.01, false, 32,
        None,
    )
    .expect("benchmark should succeed");

    assert!(output.contains("\"batchSize\":"));
    assert!(output.contains("\"totalIterations\":"));
    assert!(output.contains("\"samplesNs\":"));
}
