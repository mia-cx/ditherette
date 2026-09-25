//! Hue, saturation, and lightness in Oklab, so hue turns keep perceived lightness.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::DitheretteError;

use super::{
    chain::{check_bounded, Effect, EffectContext},
    image::EffectImage,
    space::{from_linear, linear_to_oklab, oklab_to_linear, to_linear},
};

/// Hue in degrees, saturation and lightness in `[-1, 1]`. Neutral is all zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HueSaturation {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
}

impl HueSaturation {
    fn neutral(&self) -> bool {
        self.hue == 0.0 && self.saturation == 0.0 && self.lightness == 0.0
    }

    /// Rotates and scales the Oklab opponent axes, then blends toward white or black.
    pub fn map(&self, rgb: [f32; 3]) -> [f32; 3] {
        let (sin, cos) = self.hue.to_radians().sin_cos();
        let scale = 1.0 + self.saturation;
        let [l, a, b] = linear_to_oklab(to_linear(rgb));
        let turned = [l, (a * cos - b * sin) * scale, (a * sin + b * cos) * scale];
        from_linear(oklab_to_linear(self.blend(turned)))
    }

    /// Positive lightness blends toward Oklab white `[1, 0, 0]`, negative toward black `[0, 0, 0]`.
    fn blend(&self, [l, a, b]: [f32; 3]) -> [f32; 3] {
        if self.lightness >= 0.0 {
            let keep = 1.0 - self.lightness;
            [l + (1.0 - l) * self.lightness, a * keep, b * keep]
        } else {
            let keep = 1.0 + self.lightness;
            [l * keep, a * keep, b * keep]
        }
    }
}

impl Effect for HueSaturation {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.hue, -180.0, 180.0, format!("{path}.hue"))?;
        check_bounded(self.saturation, -1.0, 1.0, format!("{path}.saturation"))?;
        check_bounded(self.lightness, -1.0, 1.0, format!("{path}.lightness"))
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        if self.neutral() {
            return;
        }
        for rgb in &mut image.rgb {
            *rgb = self.map(*rgb);
        }
    }
}
