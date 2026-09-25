//! Brightness and contrast in encoded sRGB units, pivoting contrast around mid-grey.

use serde::{Deserialize, Serialize};

use crate::spec::contract::error::DitheretteError;

use super::{
    chain::{check_bounded, Effect, EffectContext},
    image::EffectImage,
};

/// Contrast `c` scales around 0.5 by `4^c`; brightness then adds an offset. Neutral is 0 and 0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrightnessContrast {
    pub brightness: f32,
    pub contrast: f32,
}

impl BrightnessContrast {
    /// Neutral arguments return `value` untouched rather than round-tripping the pivot.
    pub fn map(&self, value: f32) -> f32 {
        if self.brightness == 0.0 && self.contrast == 0.0 {
            return value;
        }
        (value - 0.5) * 4f32.powf(self.contrast) + 0.5 + self.brightness
    }
}

impl Effect for BrightnessContrast {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.brightness, -1.0, 1.0, format!("{path}.brightness"))?;
        check_bounded(self.contrast, -1.0, 1.0, format!("{path}.contrast"))
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        for rgb in &mut image.rgb {
            *rgb = rgb.map(|value| self.map(value));
        }
    }
}
