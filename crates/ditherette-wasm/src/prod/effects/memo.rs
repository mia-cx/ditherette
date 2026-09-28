//! Direct-mapped memos for byte input whose output is a pure function of its RGB bytes.
//!
//! Analysis keeps continuous carrier values. Terminal application caches the final bytes and
//! writes them in place, so it does not allocate a full-image carrier.

use std::{collections::TryReserveError, mem::size_of};

use super::image::EffectImage;
use crate::image::ImageDimensions;

const FLOAT_BITS: u32 = 14;
const FLOAT_ENTRIES: usize = 1 << FLOAT_BITS;
const MIN_BYTE_ENTRIES: usize = 1 << 14;
const MAX_BYTE_ENTRIES: usize = 1 << 18;
const PIXELS_PER_BYTE_ENTRY: usize = 32;
/// A key that no 24-bit colour can equal.
const EMPTY: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct ByteEntry {
    key: u32,
    rgb: [u8; 3],
}

impl ByteEntry {
    const EMPTY: Self = Self {
        key: EMPTY,
        rgb: [0; 3],
    };
}

/// Bytes the analysis memo allocates alongside its continuous carrier.
pub const FLOAT_MEMO_BYTES: u64 = (FLOAT_ENTRIES * size_of::<(u32, [f32; 3])>()) as u64;

/// Number of terminal byte entries for an image, always a power of two.
pub fn byte_entries(pixels: usize) -> usize {
    pixels
        .div_ceil(PIXELS_PER_BYTE_ENTRY)
        .next_power_of_two()
        .clamp(MIN_BYTE_ENTRIES, MAX_BYTE_ENTRIES)
}

/// Bytes the terminal memo allocates for an image.
pub fn byte_memo_bytes(pixels: u64) -> u64 {
    let pixels = usize::try_from(pixels).unwrap_or(usize::MAX);
    (byte_entries(pixels) * size_of::<ByteEntry>()) as u64
}

/// Writes mapped RGB bytes in place, calling `map` once per miss. Alpha stays untouched.
/// The complete memo is allocated before the first pixel changes.
pub fn try_memoized_bytes(
    data: &mut [u8],
    map: impl FnMut([u8; 3]) -> [u8; 3],
) -> Result<(), TryReserveError> {
    try_memoized_bytes_with_entries(data, byte_entries(data.len() / 4), map)
}

fn try_memoized_bytes_with_entries(
    data: &mut [u8],
    entries: usize,
    mut map: impl FnMut([u8; 3]) -> [u8; 3],
) -> Result<(), TryReserveError> {
    debug_assert!(entries.is_power_of_two());
    let bits = entries.trailing_zeros();
    let mut memo = Vec::new();
    memo.try_reserve_exact(entries)?;
    memo.resize(entries, ByteEntry::EMPTY);
    for pixel in data.chunks_exact_mut(4) {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let key = key(rgb);
        let slot = slot(key, bits);
        if memo[slot].key != key {
            memo[slot] = ByteEntry { key, rgb: map(rgb) };
        }
        pixel[..3].copy_from_slice(&memo[slot].rgb);
    }
    Ok(())
}

/// Builds the continuous carrier, calling `map` once per distinct colour in each slot.
pub fn try_memoized(
    data: &[u8],
    dimensions: ImageDimensions,
    mut map: impl FnMut([u8; 3]) -> [f32; 3],
) -> Result<EffectImage, TryReserveError> {
    let mut memo = Vec::new();
    memo.try_reserve_exact(FLOAT_ENTRIES)?;
    memo.resize(FLOAT_ENTRIES, (EMPTY, [0.0f32; 3]));
    EffectImage::try_from_pixels(data, dimensions, |pixel| {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let key = key(rgb);
        let slot = slot(key, FLOAT_BITS);
        if memo[slot].0 != key {
            memo[slot] = (key, map(rgb));
        }
        memo[slot].1
    })
}

fn key(rgb: [u8; 3]) -> u32 {
    u32::from_le_bytes([rgb[0], rgb[1], rgb[2], 0])
}

fn slot(key: u32, bits: u32) -> usize {
    (key.wrapping_mul(0x9E37_79B1) >> (32 - bits)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_memo_scales_with_pixels() {
        assert_eq!(byte_entries(1), 1 << 14);
        assert_eq!(byte_entries(800 * 800), 1 << 15);
        assert_eq!(byte_entries(3_462 * 2_309), 1 << 18);
        assert_eq!(byte_entries(usize::MAX), 1 << 18);
        assert_eq!(size_of::<ByteEntry>(), 8);
    }

    #[test]
    fn collisions_and_evictions_keep_exact_bytes_and_alpha() {
        let entries = 4usize;
        let mut colliding = Vec::new();
        for blue in 0..=u8::MAX {
            for green in 0..=u8::MAX {
                let rgb = [17, green, blue];
                if slot(key(rgb), entries.trailing_zeros()) == 0 {
                    colliding.push(rgb);
                    if colliding.len() == 12 {
                        break;
                    }
                }
            }
            if colliding.len() == 12 {
                break;
            }
        }
        let source: Vec<u8> = colliding
            .iter()
            .cycle()
            .take(96)
            .enumerate()
            .flat_map(|(index, rgb)| [rgb[0], rgb[1], rgb[2], index as u8])
            .collect();
        let expected: Vec<u8> = source
            .chunks_exact(4)
            .flat_map(|pixel| {
                let mapped = map([pixel[0], pixel[1], pixel[2]]);
                [mapped[0], mapped[1], mapped[2], pixel[3]]
            })
            .collect();
        let mut actual = source;
        try_memoized_bytes_with_entries(&mut actual, entries, map).unwrap();
        assert_eq!(actual, expected);
    }

    fn map([red, green, blue]: [u8; 3]) -> [u8; 3] {
        [
            red.wrapping_mul(3).wrapping_add(blue),
            green.rotate_left(2),
            blue ^ red,
        ]
    }
}
