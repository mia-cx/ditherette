//! Continuous working image shared by every effect in a chain.
//!
//! RGB stays unclipped `f32` in encoded sRGB units between effects. Alpha bytes
//! ride alongside untouched. `to_rgba8` is the chain's only rounding boundary.

use crate::image::{contracts::Rgba8Image, ImageBuf, ImageDimensions, ImageView, Rgba8};

use std::{collections::TryReserveError, mem::size_of};

use super::table::ChannelTables;

/// Carrier values are bounded to `±CARRIER_LIMIT` after every step. Ordinary chains never
/// reach it; it keeps extreme ones, like 30 exposure boosts, from overflowing to infinity.
pub const CARRIER_LIMIT: f32 = 64.0;

/// Straight RGB triples in row-major order, plus the source alpha byte for each pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectImage {
    pub dimensions: ImageDimensions,
    /// Encoded sRGB units. Byte `k` enters as `k / 255`; values may leave `[0,1]`.
    pub rgb: Vec<[f32; 3]>,
    /// Source alpha, copied unchanged to the output. Effects never write it.
    pub alpha: Vec<u8>,
}

impl EffectImage {
    /// Decodes each RGB byte to `byte / 255` and keeps the alpha byte.
    pub fn from_rgba8(source: ImageView<'_, Rgba8>) -> Self {
        let dimensions = source.dimensions();
        let mut rgb = Vec::new();
        let mut alpha = Vec::new();
        for y in 0..dimensions.height() {
            let row = source.row(y).expect("source row should be in bounds");
            for pixel in row.chunks_exact(4).take(dimensions.width_usize()) {
                rgb.push([
                    unit(pixel[Rgba8::R]),
                    unit(pixel[Rgba8::G]),
                    unit(pixel[Rgba8::B]),
                ]);
                alpha.push(pixel[Rgba8::A]);
            }
        }
        Self {
            dimensions,
            rgb,
            alpha,
        }
    }

    /// Clamps every channel to `±CARRIER_LIMIT`. The executor calls it after each step.
    pub fn bound(&mut self) {
        for rgb in &mut self.rgb {
            *rgb = rgb.map(|value| value.clamp(-CARRIER_LIMIT, CARRIER_LIMIT));
        }
    }

    /// Clips each channel to `[0,1]`, scales by 255, and rounds half away from zero.
    pub fn to_rgba8(&self) -> Rgba8Image {
        let mut data = Vec::with_capacity(self.rgb.len() * 4);
        for (rgb, alpha) in self.rgb.iter().zip(&self.alpha) {
            data.extend(rgb.map(byte));
            data.push(*alpha);
        }
        ImageBuf::from_vec_packed(data, self.dimensions)
            .expect("one RGBA8 pixel is written per source pixel")
    }
}

impl EffectImage {
    /// Decodes packed RGBA8 through tabulated leading per-channel effects.
    /// Allocation failure is reported rather than aborting.
    pub fn try_from_packed(
        data: &[u8],
        dimensions: ImageDimensions,
        tables: &ChannelTables,
    ) -> Result<Self, TryReserveError> {
        Self::try_from_pixels(data, dimensions, |pixel| {
            std::array::from_fn(|channel| tables.unit(channel, pixel[channel]))
        })
    }

    /// Builds the carrier from packed RGBA8, one `rgb(pixel)` call per pixel.
    pub fn try_from_pixels(
        data: &[u8],
        dimensions: ImageDimensions,
        mut rgb_of: impl FnMut(&[u8]) -> [f32; 3],
    ) -> Result<Self, TryReserveError> {
        let pixels = data.len() / 4;
        let mut rgb = Vec::new();
        let mut alpha = Vec::new();
        rgb.try_reserve_exact(pixels)?;
        alpha.try_reserve_exact(pixels)?;
        for pixel in data.chunks_exact(4) {
            rgb.push(rgb_of(pixel));
            alpha.push(pixel[Rgba8::A]);
        }
        Ok(Self {
            dimensions,
            rgb,
            alpha,
        })
    }

    /// Bytes `try_from_packed` allocates for `pixels` pixels.
    pub const fn carrier_bytes(pixels: u64) -> u64 {
        pixels * (size_of::<[f32; 3]>() + size_of::<u8>()) as u64
    }

    /// Writes clipped, rounded RGB into packed RGBA8, leaving its alpha bytes alone.
    pub fn write_rgb(&self, data: &mut [u8]) {
        for (pixel, rgb) in data.chunks_exact_mut(4).zip(&self.rgb) {
            pixel[..3].copy_from_slice(&rgb.map(byte));
        }
    }
}

fn unit(byte: u8) -> f32 {
    byte as f32 / 255.0
}

pub(super) fn byte(unit: f32) -> u8 {
    (unit.clamp(0.0, 1.0) * 255.0).round() as u8
}
