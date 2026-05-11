use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn hello_exports_a_string() {
    assert_eq!(
        ditherette_wasm::hello("Wasm"),
        "Hello, Wasm, from Ditherette's Rust core!"
    );
}

#[wasm_bindgen_test]
fn resize_exports_rgba_bytes() {
    let source_rgba = [9, 8, 7, 255];

    let output_rgba = ditherette_wasm::resize_rgba_nearest(&source_rgba, 1, 1, 2, 1).unwrap();

    assert_eq!(output_rgba, [9, 8, 7, 255, 9, 8, 7, 255]);
}

#[wasm_bindgen_test]
fn resize_rejects_invalid_source_length() {
    let source_rgba = [1, 2, 3, 4];

    let error = ditherette_wasm::resize_rgba_nearest(&source_rgba, 2, 1, 1, 1).unwrap_err();
    let message = error.as_string().unwrap();

    assert!(message.contains("RGBA buffer length mismatch"));
}
