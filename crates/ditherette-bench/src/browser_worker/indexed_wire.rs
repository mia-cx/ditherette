//! Private capped-output transport. Decode exact bytes before the existing verifier sees a result.

use super::BrowserTransportResult;
use ditherette_bench_api::verification::{Dimensions, Pixels, VerificationOutput, Warning};
use serde::Deserialize;
use serde_json::Value;
use std::io;

const MAX_PIXELS: u64 = 8192 * 8192;
const DEFAULT_LIMIT: u64 = 64 * 1024 * 1024;
const MAX_LIMIT: u64 = 384 * 1024 * 1024;
const EVIDENCE_SLOTS: u64 = 5;
const PALETTE_AND_BOOKKEEPING: u64 = 2048;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    wire_encoding: String,
    metadata: Metadata,
    indices_hex: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Metadata {
    dimensions: WireDimensions,
    pixels: IndexedMetadata,
    warnings: Vec<WireWarning>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDimensions {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireWarning {
    code: ditherette_bench_api::verification::WarningCode,
    message: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexedMetadata {
    format: String,
    indices: Vec<u8>,
    palette_rgba: Vec<u8>,
    transparent_index: Option<u8>,
}

fn invalid() -> io::Error {
    io::Error::other("invalid capped indexed wire evidence")
}

fn decode_output(value: Value, expected: Dimensions) -> io::Result<VerificationOutput> {
    let envelope: Envelope = serde_json::from_value(value).map_err(|_| invalid())?;
    let metadata = envelope.metadata;
    let pixels = metadata.pixels;
    let count = u64::from(expected.width) * u64::from(expected.height);
    if envelope.wire_encoding != "indexed8-hex-v1"
        || metadata.dimensions.width != expected.width
        || metadata.dimensions.height != expected.height
        || expected.width == 0
        || expected.height == 0
        || count > MAX_PIXELS
        || pixels.format != "indexed8"
        || !pixels.indices.is_empty()
        || !(4..=1024).contains(&pixels.palette_rgba.len())
        || pixels.palette_rgba.len() % 4 != 0
        || pixels
            .transparent_index
            .is_some_and(|index| usize::from(index) >= pixels.palette_rgba.len() / 4)
        || metadata.warnings.len() > 3
        || metadata
            .warnings
            .iter()
            .any(|warning| warning.message.encode_utf16().count() > 88)
        || envelope.indices_hex.len() as u64 != count * 2
    {
        return Err(invalid());
    }
    let nibble = |byte| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    };
    // Validate before reserving the decoded image, including every palette index.
    for pair in envelope.indices_hex.as_bytes().chunks_exact(2) {
        let index =
            nibble(pair[0]).ok_or_else(invalid)? * 16 + nibble(pair[1]).ok_or_else(invalid)?;
        if usize::from(index) >= pixels.palette_rgba.len() / 4 {
            return Err(invalid());
        }
    }
    let indices = envelope
        .indices_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| nibble(pair[0]).unwrap() * 16 + nibble(pair[1]).unwrap())
        .collect();
    Ok(VerificationOutput {
        dimensions: expected,
        pixels: Pixels::Indexed8 {
            indices,
            palette_rgba: pixels.palette_rgba,
            transparent_index: pixels.transparent_index,
        },
        warnings: metadata
            .warnings
            .into_iter()
            .map(|warning| Warning {
                code: warning.code,
                message: warning.message,
            })
            .collect(),
    })
}

/// Only an explicitly declared capped case reaches this decoder. Ordinary JSON keeps its existing parser.
pub(super) fn decode(
    line: &str,
    expected: Dimensions,
    limit: u64,
) -> io::Result<BrowserTransportResult> {
    let count = u64::from(expected.width) * u64::from(expected.height);
    if limit <= DEFAULT_LIMIT
        || limit > MAX_LIMIT
        || count > MAX_PIXELS
        || (count + PALETTE_AND_BOOKKEEPING) * EVIDENCE_SLOTS > limit
    {
        return Err(invalid());
    }
    let mut value: Value = serde_json::from_str(line).map_err(|_| invalid())?;
    if value
        .get("prime_reference_output")
        .is_some_and(|prime| !prime.is_null())
    {
        return Err(invalid());
    }
    // Remove hex strings before deserializing ordinary metadata. Never expand bytes into JSON Values.
    let mut take = |pointer: &str| -> io::Result<Option<VerificationOutput>> {
        let Some(slot) = value.pointer_mut(pointer).filter(|slot| !slot.is_null()) else {
            return Ok(None);
        };
        let mut output = decode_output(slot.take(), expected)?;
        let Pixels::Indexed8 { indices, .. } = &mut output.pixels else {
            unreachable!()
        };
        let saved = std::mem::take(indices);
        *slot = serde_json::to_value(&output).map_err(|_| invalid())?;
        let Pixels::Indexed8 { indices, .. } = &mut output.pixels else {
            unreachable!()
        };
        *indices = saved;
        Ok(Some(output))
    };
    let output = take("/output")?.ok_or_else(invalid)?;
    let unstable = take("/unstable_output")?;
    let reference = take("/reference/output")?.ok_or_else(invalid)?;
    let mut result: BrowserTransportResult =
        serde_json::from_value(value).map_err(|_| invalid())?;
    result.output = output;
    result.unstable_output = unstable;
    result.reference.as_mut().ok_or_else(invalid)?.output = reference;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn envelope() -> Value {
        json!({"wire_encoding":"indexed8-hex-v1", "indices_hex":"0001", "metadata": {
            "dimensions":{"width":2,"height":1},
            "pixels":{"format":"indexed8","indices":[],"palette_rgba":[10,20,30,255,0,0,0,0],"transparent_index":1},
            "warnings":[{"code":"transparent-fallback","message":"fixture"}]
        }})
    }

    #[test]
    fn decodes_exact_indices_and_metadata() {
        let output = decode_output(
            envelope(),
            Dimensions {
                width: 2,
                height: 1,
            },
        )
        .unwrap();
        let Pixels::Indexed8 {
            indices,
            palette_rgba,
            transparent_index,
        } = output.pixels
        else {
            panic!()
        };
        assert_eq!(indices, [0, 1]);
        assert_eq!(palette_rgba, [10, 20, 30, 255, 0, 0, 0, 0]);
        assert_eq!(transparent_index, Some(1));
        assert_eq!(output.warnings[0].message, "fixture");
    }

    #[test]
    fn rejects_noncanonical_and_malformed_evidence_without_dumping_it() {
        for (pointer, replacement) in [
            ("/wire_encoding", json!("unknown")),
            ("/indices_hex", json!("0")),
            ("/indices_hex", json!("0002")),
            ("/indices_hex", json!("00AA")),
            ("/indices_hex", json!("00gg")),
            ("/metadata/pixels/indices", json!([0])),
            ("/metadata/pixels/transparent_index", json!(2)),
            ("/metadata/dimensions/height", json!(2)),
            ("/metadata/warnings/0/message", json!("x".repeat(89))),
            ("/metadata/warnings/0/code", json!("unknown")),
        ] {
            let mut value = envelope();
            *value.pointer_mut(pointer).unwrap() = replacement;
            let error = decode_output(
                value,
                Dimensions {
                    width: 2,
                    height: 1,
                },
            )
            .unwrap_err();
            assert!(error.to_string().len() < 100);
        }
    }

    #[test]
    fn rejects_undeclared_or_insufficient_budget_before_parsing() {
        for limit in [DEFAULT_LIMIT, MAX_LIMIT + 1, 320 * 1024 * 1024] {
            assert!(decode(
                "not json",
                Dimensions {
                    width: 8192,
                    height: 8192
                },
                limit
            )
            .is_err());
        }
    }

    #[test]
    fn restores_both_actual_outputs_and_reference_before_existing_verification() {
        let dimensions = Dimensions {
            width: 2,
            height: 1,
        };
        let mut distinct = envelope();
        distinct["indices_hex"] = json!("0100");
        let digest = [0; 32];
        let record = json!({
            "role":"accepted", "pair":0, "case_name":"tiny-capped-wire",
            "input":digest, "settings":digest, "sample_ns":[1,2,3,4,5],
            "iterations_per_sample":1, "warmup_iterations":1, "warmup_elapsed_ns":1,
            "output":distinct, "unstable_output":envelope(),
            "reference":{"case":{
                "semantics":{"operation":"process","recipe":"fixture","version":1,"space":"srgb"},
                "input":digest,"settings":digest,"output":dimensions
            },"output":envelope()},
            "observation":{"engine":"chromium","browser_version":"test","node_version":"test",
                "playwright_version":"test","user_agent":"test","cross_origin_isolated":false,"timer_resolution_ns":1}
        });
        let result = decode(&record.to_string(), dimensions, MAX_LIMIT).unwrap();
        assert_eq!(
            result.unstable_output.as_ref().unwrap(),
            &result.reference.as_ref().unwrap().output
        );
        assert_ne!(result.output, result.unstable_output.unwrap());
        assert!(crate::verification::render_rgba(&result.output)
            .unwrap()
            .is_some());
        let mut primed = record.clone();
        primed["prime_reference_output"] = envelope();
        assert!(decode(&primed.to_string(), dimensions, MAX_LIMIT).is_err());
        let mut plain = record;
        plain["output"] = serde_json::to_value(&result.output).unwrap();
        assert!(decode(&plain.to_string(), dimensions, MAX_LIMIT).is_err());
    }
}
