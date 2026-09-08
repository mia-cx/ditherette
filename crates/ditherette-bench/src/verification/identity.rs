//! Shared content and normalized-setting identities for native and Wasm verifiers.

use ditherette_bench_api::verification::{Digest256, Dimensions};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// SHA-256 of complete bytes. No shortened identifiers enter artifacts.
pub fn content_digest(bytes: &[u8]) -> Digest256 {
    Digest256(Sha256::digest(bytes).into())
}

/// Includes dimensions and every source byte, with a versioned domain prefix.
pub fn input_digest(dimensions: Dimensions, rgba: &[u8]) -> Digest256 {
    let mut hash = Sha256::new();
    hash.update(b"ditherette-rgba8-input-v1\0");
    hash.update(dimensions.width.to_le_bytes());
    hash.update(dimensions.height.to_le_bytes());
    hash.update(rgba);
    Digest256(hash.finalize().into())
}

/// Hash typed normalized settings using canonical JSON object ordering.
pub fn settings_digest<P: Serialize>(settings: &P) -> Result<Digest256, serde_json::Error> {
    Ok(content_digest(&serde_json::to_vec(&serde_json::to_value(
        settings,
    )?)?))
}
