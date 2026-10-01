//! Per-channel byte lookup: the first stage on the GPU, and the startup known-answer check.

use crate::image::ImageDimensions;

use super::Stage;

/// Maps each RGB byte through its channel's 256-entry table and keeps alpha.
///
/// This is the terminal step of a tabulated per-channel effects run
/// ([`crate::prod::effects::table::ChannelTables::bytes`]). Integer lookups
/// leave no rounding to disagree on, so the GPU must match the CPU exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelLookup(pub [[u8; 256]; 3]);

/// Width and height of the known-answer image. The odd width catches row-stride mistakes.
const KNOWN_WIDTH: u32 = 23;
const KNOWN_HEIGHT: u32 = 12;

impl ChannelLookup {
    /// A table that moves every byte of every channel, and an image whose
    /// channels each cover all 256 bytes, alpha included.
    pub fn known_answer() -> (Self, ImageDimensions, Vec<u8>) {
        let tables = Self([
            std::array::from_fn(|k| 255 - k as u8),
            std::array::from_fn(|k| (k as u8).rotate_left(3)),
            std::array::from_fn(|k| (k as u8) ^ 0xa5),
        ]);
        let dimensions = ImageDimensions::new(KNOWN_WIDTH, KNOWN_HEIGHT)
            .expect("known-answer dimensions are valid");
        let rgba = (0..KNOWN_WIDTH * KNOWN_HEIGHT)
            .flat_map(|i| [i, i * 7, i * 13 + 5, 255u32.wrapping_sub(i)].map(|v| v as u8))
            .collect();
        (tables, dimensions, rgba)
    }
}

impl Stage for ChannelLookup {
    const WGSL: &'static str = r"
@group(0) @binding(2) var<storage, read> tables: array<u32, 768>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if !inside(id.xy) {
        return;
    }
    let rgba = load(id.xy);
    store(id.xy, vec4<u32>(tables[rgba.r], tables[256u + rgba.g], tables[512u + rgba.b], rgba.a));
}
";

    fn params(&self) -> Vec<u8> {
        self.0
            .iter()
            .flatten()
            .flat_map(|&byte| u32::from(byte).to_le_bytes())
            .collect()
    }

    fn cpu(&self, rgba: &mut [u8]) {
        let [red, green, blue] = &self.0;
        for pixel in rgba.chunks_exact_mut(4) {
            pixel[0] = red[pixel[0] as usize];
            pixel[1] = green[pixel[1] as usize];
            pixel[2] = blue[pixel[2] as usize];
        }
    }
}
