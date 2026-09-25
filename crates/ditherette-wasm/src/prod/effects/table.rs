//! Tabulates a leading run of per-channel effects for byte input.
//!
//! Every pixel with channel byte `k` starts at `k / 255` and passes through the
//! same scalar maps, so one 256-entry table per channel reproduces the run exactly.

use super::{chain::ChannelFn, channel::Channel, image::byte};

/// Continuous channel values after the run, indexed by channel then input byte.
pub struct ChannelTables([[f32; 256]; 3]);

impl ChannelTables {
    /// Composes `maps` in order. An empty run yields `k / 255`, the carrier's own decode.
    pub fn new<'a>(maps: impl IntoIterator<Item = (Channel, &'a dyn ChannelFn)>) -> Self {
        let mut tables = [std::array::from_fn(|k| k as f32 / 255.0); 3];
        for (channel, map) in maps {
            for (index, table) in tables.iter_mut().enumerate() {
                if channel.selects(index) {
                    for value in table.iter_mut() {
                        *value = map.map(*value);
                    }
                }
            }
        }
        Self(tables)
    }

    pub fn unit(&self, channel: usize, input: u8) -> f32 {
        self.0[channel][input as usize]
    }

    /// Final bytes when the run is the whole chain: one clip and round per entry.
    pub fn bytes(&self) -> [[u8; 256]; 3] {
        self.0.map(|table| table.map(byte))
    }
}
