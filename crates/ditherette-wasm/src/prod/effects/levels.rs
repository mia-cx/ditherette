//! Levels: input black/white points, midtone gamma, and output black/white points.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::{check_bounded, Effect, EffectContext},
    channel::Channel,
    image::EffectImage,
};

/// Smallest and largest accepted midtone gamma.
pub const GAMMA_MIN: f32 = 0.1;
pub const GAMMA_MAX: f32 = 10.0;

/// A black/white pair in encoded sRGB units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Points {
    pub black: f32,
    pub white: f32,
}

/// Classic levels. Neutral arguments are input 0..1, gamma 1, output 0..1.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Levels {
    pub channel: Channel,
    pub input: Points,
    pub gamma: f32,
    pub output: Points,
}

impl Levels {
    /// Maps one channel value: normalize and clip the input range, shape, then scale to the output range.
    pub fn map(&self, value: f32) -> f32 {
        let t =
            ((value - self.input.black) / (self.input.white - self.input.black)).clamp(0.0, 1.0);
        let shaped = if self.gamma == 1.0 {
            t
        } else {
            t.powf(1.0 / self.gamma)
        };
        self.output.black + (self.output.white - self.output.black) * shaped
    }
}

impl Effect for Levels {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.input.black, 0.0, 1.0, format!("{path}.input.black"))?;
        check_bounded(self.input.white, 0.0, 1.0, format!("{path}.input.white"))?;
        if self.input.black >= self.input.white {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.input"),
                "Input black must be below input white.",
            ));
        }
        check_bounded(self.gamma, GAMMA_MIN, GAMMA_MAX, format!("{path}.gamma"))?;
        check_bounded(self.output.black, 0.0, 1.0, format!("{path}.output.black"))?;
        check_bounded(self.output.white, 0.0, 1.0, format!("{path}.output.white"))
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        for rgb in &mut image.rgb {
            self.channel.apply(rgb, |value| self.map(value));
        }
    }
}
