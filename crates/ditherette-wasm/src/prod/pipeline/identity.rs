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

pub(super) fn palette(
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

pub(super) fn resize(width: u32, height: u32, output: Output) -> Result<Identity, Failure> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::contract::{cache as frozen, request};

    #[test]
    fn streaming_keys_match_frozen_normalized_settings() {
        let palettes = [
            vec![
                PaletteEntry::Color { rgb: [1, 2, 3] },
                PaletteEntry::Transparent {},
            ],
            vec![
                PaletteEntry::Transparent {},
                PaletteEntry::Color { rgb: [1, 2, 3] },
            ],
            vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 256],
            vec![PaletteEntry::Color { rgb: [1, 2, 3] }; 257],
        ];
        for entries in &palettes {
            for matching in [
                "srgb-euclidean",
                "srgb-compuphase",
                "srgb-rec601",
                "srgb-rec709",
                "linear-rgb-euclidean",
                "oklab-euclidean",
                "oklch-euclidean",
                "oklch-circular-hue",
                "oklch-hue-arc",
                "cielab-euclidean",
                "cielab-ciede2000",
                "cielch-euclidean",
                "cielch-circular-hue",
                "cielch-hue-arc",
                "ycbcr-euclidean",
            ] {
                for alpha in [
                    AlphaPolicy::Preserve { threshold: -0.0 },
                    AlphaPolicy::Preserve { threshold: 0.0 },
                    AlphaPolicy::Preserve {
                        threshold: 127.9999999,
                    },
                    AlphaPolicy::Preserve { threshold: 128.0 },
                    AlphaPolicy::Premultiplied {},
                    AlphaPolicy::Matte { rgb: [3, 19, 47] },
                ] {
                    let matching: MatchPolicy =
                        serde_json::from_value(serde_json::json!(matching)).unwrap();
                    let expected = frozen::stage_identity(
                        None,
                        RECIPE_VERSION,
                        frozen::StageOptions::PreparedPalette {
                            palette: frozen::palette_identity(entries),
                            alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap())
                                .unwrap(),
                            matching: serde_json::from_value(
                                serde_json::to_value(matching).unwrap(),
                            )
                            .unwrap(),
                        },
                    );
                    assert_eq!(palette(entries, alpha, matching).unwrap().0, expected.0);
                }
            }
        }
        for algorithm in [
            "nearest",
            "area",
            "bilinear",
            "bicubic",
            "lanczos2",
            "lanczos3",
            "trilinear",
        ] {
            for anchor in [
                "top-left",
                "top",
                "top-right",
                "left",
                "center",
                "right",
                "bottom-left",
                "bottom",
                "bottom-right",
            ] {
                for support in ["fixed", "scale-aware"] {
                    let mut settings = serde_json::json!({"algorithm": algorithm});
                    if algorithm != "area" {
                        settings["anchor"] = serde_json::json!(anchor);
                    }
                    if ["bicubic", "lanczos2", "lanczos3"].contains(&algorithm) {
                        settings["support"] = serde_json::json!(support);
                    }
                    let output = Output {
                        width: 17,
                        height: 5,
                        resize: serde_json::from_value(settings).unwrap(),
                    };
                    let expected = frozen::stage_identity(
                        None,
                        RECIPE_VERSION,
                        frozen::StageOptions::ResizePlan {
                            source_width: 31,
                            source_height: 9,
                            output: serde_json::from_value::<request::Output>(
                                serde_json::to_value(output).unwrap(),
                            )
                            .unwrap(),
                        },
                    );
                    assert_eq!(resize(31, 9, output).unwrap().0, expected.0);
                }
            }
        }
    }
}
