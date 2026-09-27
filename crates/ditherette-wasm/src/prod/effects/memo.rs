//! Direct-mapped memo for byte input whose carrier value is a pure function of its RGB bytes.
//!
//! After a tabulated run, every pixel with the same bytes enters an effect with the same
//! carrier value, so its output repeats too. The memo checks the full key on every hit.

use std::{collections::TryReserveError, mem::size_of};

use super::image::EffectImage;
use crate::image::ImageDimensions;

const BITS: u32 = 14;
const ENTRIES: usize = 1 << BITS;
/// A key that no 24-bit colour can equal.
const EMPTY: u32 = u32::MAX;

/// Bytes one memoized pass allocates, charged like the carrier.
pub const MEMO_BYTES: u64 = (ENTRIES * size_of::<(u32, [f32; 3])>()) as u64;

/// Builds the carrier from packed RGBA8, calling `map` once per distinct colour in each slot.
pub fn try_memoized(
    data: &[u8],
    dimensions: ImageDimensions,
    mut map: impl FnMut([u8; 3]) -> [f32; 3],
) -> Result<EffectImage, TryReserveError> {
    let mut memo = Vec::new();
    memo.try_reserve_exact(ENTRIES)?;
    memo.resize(ENTRIES, (EMPTY, [0.0f32; 3]));
    EffectImage::try_from_pixels(data, dimensions, |pixel| {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let key = u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 0]);
        let slot = (key.wrapping_mul(0x9E37_79B1) >> (32 - BITS)) as usize;
        if memo[slot].0 != key {
            memo[slot] = (key, map(rgb));
        }
        memo[slot].1
    })
}
