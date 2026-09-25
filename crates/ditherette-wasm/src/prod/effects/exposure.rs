//! Exposure in photographic stops, applied in linear light.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::DitheretteError;

use super::space::{linear_to_srgb_unit, srgb_unit_to_linear};
use super::{
    chain::{check_bounded, Effect, EffectContext},
    image::EffectImage,
};

/// Largest accepted exposure change, in stops either way.
pub const MAX_STOPS: f32 = 4.0;

/// Multiplies linear light by `2^stops`. Neutral is 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exposure {
    pub stops: f32,
}

impl Exposure {
    pub fn map(&self, value: f32) -> f32 {
        if self.stops == 0.0 {
            return value;
        }
        linear_to_srgb_unit(srgb_unit_to_linear(value) * 2f32.powf(self.stops))
    }
}

impl Effect for Exposure {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(
            self.stops,
            -MAX_STOPS,
            MAX_STOPS,
            format_args!("{path}.stops"),
        )
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        for rgb in &mut image.rgb {
            *rgb = rgb.map(|value| self.map(value));
        }
    }

    fn per_channel(&self) -> bool {
        true
    }

    fn map_channel(&self, _channel: usize, value: f32) -> f32 {
        self.map(value)
    }
}
