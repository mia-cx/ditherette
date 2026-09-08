//! Allocation-free canonical preparation identities matching the frozen JSON recipe.

use std::io::{self, Write};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    image::contracts::PaletteEntry,
    prod::contract::{
        cache::Identity,
        error::ErrorCode,
        failure::{ErrorPath, Failure},
        request::{
            AlphaPolicy, MatchPolicy, Output, ResizePolicy, MAX_PALETTE_ENTRIES, RECIPE_VERSION,
        },
    },
};

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash(domain: &[u8], value: &impl Serialize) -> Result<Identity, Failure> {
    let mut writer = HashWriter(Sha256::new());
    writer.0.update(domain);
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
    Ok(Identity(writer.0.finalize().into()))
}

// Field order deliberately matches serde_json::Value's sorted object keys.
#[derive(Serialize)]
struct Stage<T> {
    options: T,
    parent: Option<Identity>,
    version: u32,
}

/// Hashes validated palette and matching preparation settings without heap allocation.
pub fn palette(
    entries: &[PaletteEntry],
    alpha: AlphaPolicy,
    matching: MatchPolicy,
) -> Result<Identity, Failure> {
    #[derive(Serialize)]
    struct Palette<'a> {
        retained: &'a [PaletteEntry],
        truncated: bool,
    }
    #[derive(Serialize)]
    struct Options {
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        palette: Identity,
        stage: &'static str,
    }
    let palette = hash(
        b"ditherette-palette-v1\0",
        &Palette {
            retained: &entries[..entries.len().min(MAX_PALETTE_ENTRIES)],
            truncated: entries.len() > MAX_PALETTE_ENTRIES,
        },
    )?;
    let alpha = match alpha {
        AlphaPolicy::Preserve { threshold } if threshold == 0.0 => {
            AlphaPolicy::Preserve { threshold: 0.0 }
        }
        alpha => alpha,
    };
    hash(
        b"ditherette-stage-v1\0",
        &Stage {
            options: Options {
                alpha,
                matching,
                palette,
                stage: "prepared-palette",
            },
            parent: None,
            version: RECIPE_VERSION,
        },
    )
}

/// Hashes validated dimensions and the complete resize recipe without image bytes.
pub fn resize(width: u32, height: u32, output: Output) -> Result<Identity, Failure> {
    #[derive(Serialize)]
    struct SortedOutput {
        height: u32,
        resize: ResizePolicy,
        width: u32,
    }
    #[derive(Serialize)]
    struct Options {
        output: SortedOutput,
        source_height: u32,
        source_width: u32,
        stage: &'static str,
    }
    hash(
        b"ditherette-stage-v1\0",
        &Stage {
            options: Options {
                output: SortedOutput {
                    height: output.height,
                    resize: output.resize,
                    width: output.width,
                },
                source_height: height,
                source_width: width,
                stage: "resize-plan",
            },
            parent: None,
            version: RECIPE_VERSION,
        },
    )
}
