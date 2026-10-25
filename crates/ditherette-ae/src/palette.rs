//! Palettes: the editor's default, or every colour found in a swatch layer.

use std::collections::HashSet;

use ditherette_wasm::{
    image::contracts::PaletteEntry, prod::contract::request::MAX_PALETTE_ENTRIES,
};

use crate::render::Image;

/// Wplace (Default) from `src/lib/palette/wplace.ts`: free, then premium, then Transparent.
const WPLACE: [u32; 63] = [
    0x000000, 0x3C3C3C, 0x787878, 0xD2D2D2, 0xFFFFFF, 0x600018, 0xED1C24, 0xFF7F27, 0xF6AA09,
    0xF9DD3B, 0xFFFABC, 0x0EB968, 0x13E67B, 0x87FF5E, 0x0C816E, 0x10AEA6, 0x13E1BE, 0x60F7F2,
    0x28509E, 0x4093E4, 0x6B50F6, 0x99B1FB, 0x780C99, 0xAA38B9, 0xE09FF9, 0xCB007A, 0xEC1F80,
    0xF38DA9, 0x684634, 0x95682A, 0xF8B277, 0xAAAAAA, 0xA50E1E, 0xFA8072, 0xE45C1A, 0x9C8431,
    0xC5AD31, 0xE8D45F, 0x4A6B3A, 0x5A944A, 0x84C573, 0x0F799F, 0xBBFAF2, 0x7DC7FF, 0x4D31B8,
    0x4A4284, 0x7A71C4, 0xB5AEF1, 0x9B5249, 0xD18078, 0xFAB6A4, 0xDBA463, 0x7B6352, 0x9C846B,
    0xD6B594, 0xD18051, 0xFFC5A5, 0x6D643F, 0x948C6B, 0xCDC59E, 0x333941, 0x6D758D, 0xB3B9D1,
];

/// The palette used when no palette layer is chosen.
pub fn wplace() -> Vec<PaletteEntry> {
    WPLACE
        .iter()
        .map(|&hex| PaletteEntry::Color {
            rgb: [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8],
        })
        .chain([PaletteEntry::Transparent {}])
        .collect()
}

/// Each distinct fully opaque colour of `layer` in reading order, then Transparent
/// so the target layer keeps its alpha. Partly transparent pixels are edges, not swatches.
pub fn from_layer(layer: Image<'_>) -> Vec<PaletteEntry> {
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    'rows: for y in 0..layer.height {
        for [alpha, red, green, blue] in layer.row(y).as_chunks::<4>().0.iter().copied() {
            if alpha == u8::MAX && seen.insert([red, green, blue]) {
                entries.push(PaletteEntry::Color {
                    rgb: [red, green, blue],
                });
                // Keep the last slot for Transparent.
                if entries.len() == MAX_PALETTE_ENTRIES - 1 {
                    break 'rows;
                }
            }
        }
    }
    entries.push(PaletteEntry::Transparent {});
    entries
}
