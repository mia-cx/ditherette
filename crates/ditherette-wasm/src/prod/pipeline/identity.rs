//! Allocation-free canonical semantic identities matching the frozen JSON recipe.

use std::io::{self, Write};

use serde::{ser::SerializeMap, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::{
    image::contracts::PaletteEntry,
    prod::contract::{
        cache::{Identity, StageOptions},
        error::ErrorCode,
        failure::{ErrorPath, Failure},
        request::{
            AlphaPolicy, DitherPolicy, MatchPolicy, Output, PerturbPolicy, Placement, ResizePolicy,
            MAX_PALETTE_ENTRIES, RECIPE_VERSION,
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
    struct Options {
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        palette: Identity,
        stage: &'static str,
    }
    let palette = palette_content(entries)?;
    let alpha = normalized_alpha(alpha);
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

/// Ordered palette content, including the warning-affecting truncation bit.
pub fn palette_content(entries: &[PaletteEntry]) -> Result<Identity, Failure> {
    #[derive(Serialize)]
    struct Palette<'a> {
        retained: &'a [PaletteEntry],
        truncated: bool,
    }
    hash(
        b"ditherette-palette-v1\0",
        &Palette {
            retained: &entries[..entries.len().min(MAX_PALETTE_ENTRIES)],
            truncated: entries.len() > MAX_PALETTE_ENTRIES,
        },
    )
}

fn normalized_alpha(alpha: AlphaPolicy) -> AlphaPolicy {
    match alpha {
        AlphaPolicy::Preserve { threshold } if threshold == 0.0 => {
            AlphaPolicy::Preserve { threshold: 0.0 }
        }
        alpha => alpha,
    }
}

#[derive(Serialize)]
struct SortedOutput {
    height: u32,
    resize: ResizePolicy,
    width: u32,
}

impl From<Output> for SortedOutput {
    fn from(output: Output) -> Self {
        Self {
            height: output.height,
            resize: output.resize,
            width: output.width,
        }
    }
}

// The frozen Value serializer promotes f32 to f64 before encoding and normalizes signed zero.
fn normalized_float(value: f32) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        f64::from(value)
    }
}

struct SortedPlacement(Placement);
impl Serialize for SortedPlacement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        match self.0 {
            Placement::Everywhere {} => map.serialize_entry("mode", "everywhere")?,
            Placement::Adaptive {
                radius,
                threshold,
                softness,
            } => {
                map.serialize_entry("mode", "adaptive")?;
                map.serialize_entry("radius", &radius)?;
                map.serialize_entry("softness", &normalized_float(softness))?;
                map.serialize_entry("threshold", &normalized_float(threshold))?;
            }
        }
        map.end()
    }
}

struct SortedPerturb(PerturbPolicy);
impl Serialize for SortedPerturb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("field", &self.0.field)?;
        map.serialize_entry("placement", &SortedPlacement(self.0.placement))?;
        map.serialize_entry("space", &self.0.space)?;
        map.serialize_entry("strength", &normalized_float(self.0.strength))?;
        map.end()
    }
}

struct SortedDither(DitherPolicy);
impl Serialize for SortedDither {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        match self.0 {
            DitherPolicy::None {} => map.serialize_entry("family", "none")?,
            DitherPolicy::Separable { perturb } => {
                map.serialize_entry("family", "separable")?;
                map.serialize_entry("perturb", &SortedPerturb(perturb))?;
            }
            DitherPolicy::Diffusion {
                kernel,
                strength,
                placement,
                serpentine,
                feedback,
            } => {
                map.serialize_entry("family", "diffusion")?;
                map.serialize_entry("feedback", &feedback)?;
                map.serialize_entry("kernel", &kernel)?;
                map.serialize_entry("placement", &SortedPlacement(placement))?;
                map.serialize_entry("serpentine", &serpentine)?;
                map.serialize_entry("strength", &normalized_float(strength))?;
            }
            DitherPolicy::Yliluoma { size, placement } => {
                map.serialize_entry("family", "yliluoma")?;
                map.serialize_entry("placement", &SortedPlacement(placement))?;
                map.serialize_entry("size", &size)?;
            }
        }
        map.end()
    }
}

struct SortedStage(StageOptions);
impl Serialize for SortedStage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        match self.0 {
            StageOptions::Resize { output } => {
                map.serialize_entry("output", &SortedOutput::from(output))?;
                map.serialize_entry("stage", "resize")?;
            }
            StageOptions::Perturb { perturb } => {
                map.serialize_entry("perturb", &SortedPerturb(perturb))?;
                map.serialize_entry("stage", "perturb")?;
            }
            StageOptions::Color { space } => {
                map.serialize_entry("space", &space)?;
                map.serialize_entry("stage", "color")?;
            }
            StageOptions::Alpha { palette, alpha } => {
                map.serialize_entry("alpha", &normalized_alpha(alpha))?;
                map.serialize_entry("palette", &palette)?;
                map.serialize_entry("stage", "alpha")?;
            }
            StageOptions::Indexed {
                palette,
                alpha,
                matching,
                dither,
            } => {
                map.serialize_entry("alpha", &normalized_alpha(alpha))?;
                map.serialize_entry("dither", &SortedDither(dither))?;
                map.serialize_entry("matching", &matching)?;
                map.serialize_entry("palette", &palette)?;
                map.serialize_entry("stage", "indexed")?;
            }
            StageOptions::PreparedPalette {
                palette,
                alpha,
                matching,
            } => {
                map.serialize_entry("alpha", &normalized_alpha(alpha))?;
                map.serialize_entry("matching", &matching)?;
                map.serialize_entry("palette", &palette)?;
                map.serialize_entry("stage", "prepared-palette")?;
            }
            StageOptions::ResizePlan {
                source_width,
                source_height,
                output,
            } => {
                map.serialize_entry("output", &SortedOutput::from(output))?;
                map.serialize_entry("source_height", &source_height)?;
                map.serialize_entry("source_width", &source_width)?;
                map.serialize_entry("stage", "resize-plan")?;
            }
        }
        map.end()
    }
}

/// Hash a validated semantic operation with the exact frozen canonical JSON framing.
pub fn stage(parent: Option<Identity>, options: StageOptions) -> Result<Identity, Failure> {
    hash(
        b"ditherette-stage-v1\0",
        &Stage {
            options: SortedStage(options),
            parent,
            version: RECIPE_VERSION,
        },
    )
}

/// Shared Alpha → Color → Indexed composition, independent of the calling public method.
pub fn indexed(
    parent: Identity,
    palette: Identity,
    alpha: AlphaPolicy,
    matching: MatchPolicy,
    dither: DitherPolicy,
) -> Result<Identity, Failure> {
    let alpha_key = stage(Some(parent), StageOptions::Alpha { palette, alpha })?;
    let color_key = stage(
        Some(alpha_key),
        StageOptions::Color {
            space: matching.space(),
        },
    )?;
    stage(
        Some(color_key),
        StageOptions::Indexed {
            palette,
            alpha,
            matching,
            dither,
        },
    )
}

/// Hashes validated dimensions and the complete resize recipe without image bytes.
pub fn resize(width: u32, height: u32, output: Output) -> Result<Identity, Failure> {
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
