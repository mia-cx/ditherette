//! Tabulates a leading run of per-channel effects for byte input.
//!
//! Every pixel with channel byte `k` starts at `k / 255` and passes through the
//! same scalar maps, so one 256-entry table per channel reproduces the run exactly.

use super::{chain::Effect, image::byte};

/// Continuous channel values after the run, indexed by channel then input byte.
pub struct ChannelTables([[f32; 256]; 3]);

impl ChannelTables {
    /// Composes per-channel `effects` in order. An empty run yields `k / 255`, the carrier's own decode.
    pub fn new<'a, E: Effect + 'a>(effects: impl IntoIterator<Item = &'a E>) -> Self {
        let mut tables = [std::array::from_fn(|k| k as f32 / 255.0); 3];
        for effect in effects {
            for (channel, table) in tables.iter_mut().enumerate() {
                for value in table.iter_mut() {
                    *value = effect.map_channel(channel, *value);
                }
            }
        }
        Self(tables)
    }

    pub fn unit(&self, channel: usize, input: u8) -> f32 {
        self.0[channel][input as usize]
    }

    /// Every entry passed through `map`, for effects that start with a per-channel transform.
    pub fn map(&self, map: impl Fn(f32) -> f32) -> Self {
        Self(self.0.map(|table| table.map(&map)))
    }

    /// Final bytes when the run is the whole chain: one clip and round per entry.
    pub fn bytes(&self) -> [[u8; 256]; 3] {
        self.0.map(|table| table.map(byte))
    }
}
