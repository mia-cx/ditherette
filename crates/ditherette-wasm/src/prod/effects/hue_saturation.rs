//! Hue, saturation, and lightness in Oklab, so hue turns keep perceived lightness.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::DitheretteError;

use super::{
    chain::{check_bounded, Effect, EffectContext, PixelMap},
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

#[derive(Clone, Copy)]
struct Turn {
    sin: f32,
    cos: f32,
    scale: f32,
}

impl HueSaturation {
    fn neutral(&self) -> bool {
        self.hue == 0.0 && self.saturation == 0.0 && self.lightness == 0.0
    }

    /// The turn's sine, cosine, and chroma scale, computed once per call instead of per pixel.
    fn turn(&self) -> Turn {
        let (sin, cos) = self.hue.to_radians().sin_cos();
        Turn {
            sin,
            cos,
            scale: 1.0 + self.saturation,
        }
    }

    /// Rotates and scales the Oklab opponent axes of linear RGB, then blends toward white or black.
    fn map_linear(&self, turn: Turn, linear: [f32; 3]) -> [f32; 3] {
        let [l, a, b] = linear_to_oklab(linear);
        let turned = [
            l,
            (a * turn.cos - b * turn.sin) * turn.scale,
            (a * turn.sin + b * turn.cos) * turn.scale,
        ];
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
        let turn = self.turn();
        for rgb in &mut image.rgb {
            *rgb = self.map_linear(turn, to_linear(*rgb));
        }
    }

    fn pixel_map<'s>(&'s self, _context: &'s EffectContext<'_>) -> Option<PixelMap<'s>> {
        if self.neutral() {
            return Some(Box::new(|rgb| rgb));
        }
        let turn = self.turn();
        Some(Box::new(move |rgb| self.map_linear(turn, to_linear(rgb))))
    }
}
