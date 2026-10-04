//! Browser half of the GPU parity harness. Built only with the `gpu` feature.
//!
//! `scripts/gpu-parity-browser.mjs` drives it in Chromium. Nothing in the package calls it.

use wasm_bindgen::prelude::*;
use wgpu::web_sys::ImageBitmap;

use crate::prod::gpu::{Adapters, Backend, ChannelLookup, Gpu, Parity, Stage};

/// Runs the known-answer channel lookup on `bitmap` with the GPU, uploaded by
/// `copyExternalImageToTexture`, and on `rgba`, the same pixels, with the CPU.
/// Create the bitmap with `premultiplyAlpha: "none"` and `colorSpaceConversion: "none"`.
///
/// Returns JSON with the adapter and the parity, or why the CPU would run instead.
#[wasm_bindgen(js_name = gpuParity)]
pub async fn gpu_parity(bitmap: ImageBitmap, mut rgba: Vec<u8>) -> Result<String, JsError> {
    let pixels = bitmap.width() as usize * bitmap.height() as usize;
    if rgba.len() != pixels * 4 {
        return Err(JsError::new(
            "rgba must hold the bitmap's pixels as packed RGBA8",
        ));
    }
    let gpu = match Backend::detect_on(&Gpu::instance(), Adapters::Any).await {
        Backend::Gpu(gpu) => gpu,
        Backend::Cpu(reason) => {
            return Ok(serde_json::json!({ "fallback": format!("{reason:?}") }).to_string())
        }
    };
    let info = gpu.info();
    let adapter = format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type);
    let (stage, _, _) = ChannelLookup::known_answer();
    let result = async { gpu.run(&stage, &gpu.upload_bitmap(&bitmap)?).await };
    let report = match result.await {
        Ok(output) => {
            stage.cpu(&mut rgba);
            let parity = Parity::between(&rgba, &output);
            serde_json::json!({
                "adapter": adapter,
                "maxByteDifference": parity.max_byte_difference,
                "differingPixels": parity.differing_pixels,
                "pixels": parity.pixels,
            })
        }
        Err(reason) => serde_json::json!({ "adapter": adapter, "fallback": format!("{reason:?}") }),
    };
    Ok(report.to_string())
}
