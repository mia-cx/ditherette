//! White balance: temperature and tint as linear-light channel gains.

use serde::{Deserialize, Serialize};

use crate::spec::contract::error::DitheretteError;

use super::{
    chain::{check_bounded, Effect, EffectContext},
    image::EffectImage,
};
use crate::spec::color::{linear_to_srgb_unit, srgb_unit_to_linear};

/// Warmer temperature raises red and lowers blue; positive tint lowers green toward magenta.
/// At either end each gain is half a stop. Neutral is 0 and 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WhiteBalance {
    pub temperature: f32,
    pub tint: f32,
}

impl WhiteBalance {
    /// Linear-light gains for red, green, and blue.
    pub fn gains(&self) -> [f32; 3] {
        [
            2f32.powf(0.5 * self.temperature),
            2f32.powf(-0.5 * self.tint),
            2f32.powf(-0.5 * self.temperature),
        ]
    }

    /// Maps one channel. A unit gain returns the value untouched.
    pub fn map_channel(&self, channel: usize, value: f32) -> f32 {
        let gain = self.gains()[channel];
        if gain == 1.0 {
            return value;
        }
        linear_to_srgb_unit(srgb_unit_to_linear(value) * gain)
    }
}

impl Effect for WhiteBalance {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.temperature, -1.0, 1.0, format!("{path}.temperature"))?;
        check_bounded(self.tint, -1.0, 1.0, format!("{path}.tint"))
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        for rgb in &mut image.rgb {
            *rgb = std::array::from_fn(|channel| self.map_channel(channel, rgb[channel]));
        }
    }
}
