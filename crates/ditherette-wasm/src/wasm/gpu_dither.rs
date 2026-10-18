//! Async indexed jobs share the artifact's single verified device attempt.

use super::{
    fields::parse_dither,
    gpu_effects::BACKEND,
    processor::{copy_input, JsBoundary},
    quantize::{parse_alpha, parse_matching, read_palette, PALETTE_SLOTS},
};
use crate::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::request::*,
        gpu::{Backend, Fallback, IndexedMemory},
        pipeline::quantize::{IndexedMetadataRef, QuantizeBoundary},
    },
};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/wasm/gpu_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = checkJob)]
    fn check_job(sink: &JsValue, completed: u32) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = completeCoverage)]
    fn complete_coverage(bytes: &[u8], sink: &JsValue) -> Result<(), JsValue>;
}

/// Numeric-only image admission precedes the input snapshot and any device job.
#[wasm_bindgen(js_name = privateGpuDitherPlan)]
pub fn private_gpu_dither_plan(width: u32, height: u32) -> Result<f64, JsError> {
    let dimensions =
        ImageDimensions::new(width, height).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(IndexedMemory::new(dimensions).required_bytes as f64)
}

/// Palette-size admission precedes the snapshot and palette-tree allocation.
#[wasm_bindgen(js_name = privateGpuYliluomaPlan)]
pub fn private_gpu_yliluoma_plan(
    width: u32,
    height: u32,
    entries: u32,
    size: u32,
) -> Result<f64, JsError> {
    if entries > 256 || ![2, 4, 8, 16].contains(&size) {
        return Err(JsError::new("invalid mixture dimensions"));
    }
    let dimensions =
        ImageDimensions::new(width, height).map_err(|error| JsError::new(&error.to_string()))?;
    Ok(crate::prod::gpu::MixMemory::new(dimensions, entries, size).required_bytes as f64)
}

/// Verifies indexed readback and complete coverage before accepting image jobs.
#[wasm_bindgen(js_name = privateGpuDitherDetect)]
pub async fn private_gpu_dither_detect(memory_limit: f64) -> String {
    let dimensions = ImageDimensions::new(2, 2).unwrap();
    if IndexedMemory::new(dimensions).required_bytes > memory_limit as u64 {
        return "MemoryLimit".into();
    }
    let reason = super::gpu_effects::private_gpu_effects_detect(memory_limit).await;
    if !reason.is_empty() {
        return reason;
    }
    let mut backend = BACKEND
        .with(|slot| slot.borrow_mut().take())
        .expect("detected backend");
    let result = async {
        let rgba = [
            0, 0, 0, 255, 255, 255, 255, 255, 32, 32, 32, 255, 200, 200, 200, 0,
        ];
        let palette = [
            PaletteEntry::Color { rgb: [0; 3] },
            PaletteEntry::Color { rgb: [255; 3] },
            PaletteEntry::Transparent {},
        ];
        let indexed = backend
            .dither_and_quantize(
                DitherQuantizeRequest {
                    quantize: QuantizeRequest {
                        version: 1,
                        source: Source {
                            width: 2,
                            height: 2,
                            data: &rgba,
                        },
                        palette: &palette,
                        alpha: AlphaPolicy::Preserve { threshold: 127.0 },
                        matching: MatchPolicy::SrgbEuclidean,
                    },
                    dither: DitherPolicy::None {},
                },
                memory_limit as u64,
            )
            .await?;
        if indexed.indices.data() != [0, 1, 0, 2] {
            return Err(Fallback::Mismatch);
        }
        let coverage = backend
            .adaptive_coverage(dimensions, &[128; 16], 4, 0.12, memory_limit as u64)
            .await?;
        if coverage != [15, 0, 0, 0] {
            return Err(Fallback::Mismatch);
        }
        let resized = backend
            .resize(
                ResizeRequest {
                    version: 1,
                    source: Source {
                        width: 2,
                        height: 2,
                        data: &rgba,
                    },
                    output: Output {
                        width: 4,
                        height: 2,
                        resize: ResizePolicy::Nearest {
                            anchor: Anchor::Center,
                        },
                    },
                },
                false,
                crate::prod::gpu::GpuTiling::Auto,
                memory_limit as u64,
            )
            .await?;
        let expected = rgba
            .chunks_exact(4)
            .flat_map(|pixel| pixel.iter().chain(pixel).copied())
            .collect::<Vec<_>>();
        if resized != expected {
            return Err(Fallback::Mismatch);
        }
        Ok::<_, Fallback>(())
    }
    .await;
    let reason = match result {
        Ok(()) => String::new(),
        Err(reason) => {
            backend = Backend::Cpu(reason.clone());
            format!("{reason:?}")
        }
    };
    BACKEND.with(|slot| *slot.borrow_mut() = Some(backend));
    reason
}

/// Inputs use the existing numeric private recipe vocabulary; no JSON decoder
/// or caller-owned source is retained over the first await.
#[wasm_bindgen(js_name = privateGpuDitherAndQuantize)]
pub async fn private_gpu_dither_and_quantize(
    input: Uint8Array,
    width: u32,
    height: u32,
    palette: JsValue,
    matching: f64,
    alpha_mode: f64,
    alpha_threshold: f64,
    matte: f64,
    family: f64,
    field: f64,
    parameter: f64,
    space: f64,
    strength: f64,
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
    coverage: Option<Uint8Array>,
    retain: bool,
    memory_limit: f64,
    sink: JsValue,
) -> Result<String, JsError> {
    let required = private_gpu_dither_plan(width, height)?;
    if required > memory_limit {
        return Ok("MemoryLimit".into());
    }
    let dither = parse_dither(
        family, field, parameter, space, strength, placement, radius, threshold, softness,
    )
    .map_err(|e| JsError::new(&format!("invalid dither: {e:?}")))?;
    let matching =
        parse_matching(matching).map_err(|e| JsError::new(&format!("invalid matching: {e:?}")))?;
    let alpha = parse_alpha(alpha_mode, alpha_threshold, matte)
        .map_err(|e| JsError::new(&format!("invalid alpha: {e:?}")))?;
    let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
    let palette_len = read_palette(&palette, &mut entries)
        .map_err(|e| JsError::new(&format!("invalid palette: {e:?}")))?;
    if let DitherPolicy::Yliluoma { size, .. } = dither {
        let size = match size {
            BayerSize::Two => 2,
            BayerSize::Four => 4,
            BayerSize::Eight => 8,
            BayerSize::Sixteen => 16,
        };
        // The optional tail marks truncation; it is not a retained mixture colour.
        if private_gpu_yliluoma_plan(width, height, palette_len.min(256) as u32, size)?
            > memory_limit
        {
            return Ok("MemoryLimit".into());
        }
    }
    let bytes = u64::from(width) * u64::from(height) * 4;
    if bytes != u64::from(input.length()) {
        return Err(JsError::new("source dimensions no longer match"));
    }
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes as usize)
        .map_err(|_| JsError::new("source allocation failed"))?;
    rgba.resize(bytes as usize, 0);
    copy_input(&mut rgba, &input).map_err(|_| JsError::new("source copy failed"))?;
    let coverage = if let Some(input) = coverage {
        let dimensions =
            ImageDimensions::new(width, height).map_err(|e| JsError::new(&e.to_string()))?;
        let bytes = crate::prod::gpu::AreaMemory::new(dimensions).coverage_bytes as usize;
        if input.length() as usize != bytes {
            return Err(JsError::new("coverage dimensions no longer match"));
        }
        let mut bits = Vec::new();
        bits.try_reserve_exact(bytes)
            .map_err(|_| JsError::new("coverage allocation failed"))?;
        bits.resize(bytes, 0);
        copy_input(&mut bits, &input).map_err(|_| JsError::new("coverage copy failed"))?;
        Some(bits)
    } else {
        None
    };
    let mut backend = BACKEND.with(|slot| slot.borrow_mut().take());
    struct Control<'a> {
        sink: &'a JsValue,
        width: u32,
    }
    impl crate::prod::gpu::JobControl for Control<'_> {
        fn check(&mut self) -> Result<(), Fallback> {
            self.row(0)
        }
        fn row(&mut self, completed: u32) -> Result<(), Fallback> {
            match check_job(self.sink, completed * self.width) {
                Ok(true) => Ok(()),
                Ok(false) => Err(Fallback::Cancelled),
                Err(_) => Err(Fallback::Callback),
            }
        }
    }
    let mut check = Control { sink: &sink, width };
    let result = match backend.as_mut() {
        Some(backend) => {
            backend
                .dither_prepared_checked(
                    DitherQuantizeRequest {
                        quantize: QuantizeRequest {
                            version: 1,
                            source: Source {
                                width,
                                height,
                                data: &rgba,
                            },
                            palette: &entries[..palette_len],
                            alpha,
                            matching,
                        },
                        dither,
                    },
                    memory_limit as u64,
                    coverage,
                    retain,
                    &mut check,
                )
                .await
        }
        None => Err(Fallback::NoAdapter),
    };
    BACKEND.with(|slot| *slot.borrow_mut() = backend);
    match result {
        Ok((output, coverage)) => {
            crate::prod::gpu::JobControl::check(&mut check)
                .map_err(|reason| JsError::new(&format!("{reason:?}")))?;
            JsBoundary::new(&input, &sink)
                .and_then(|mut boundary| {
                    boundary.complete(
                        output.indices.data(),
                        output.indices.dimensions(),
                        IndexedMetadataRef {
                            palette: &output.palette,
                            warnings: &output.warnings,
                        },
                    )
                })
                .map_err(|_| JsError::new("indexed result copy failed"))?;
            if let Some(bits) = coverage {
                complete_coverage(&bits, &sink)
                    .map_err(|_| JsError::new("coverage result copy failed"))?;
            }
            Ok(String::new())
        }
        Err(reason) => Ok(format!("{reason:?}")),
    }
}
