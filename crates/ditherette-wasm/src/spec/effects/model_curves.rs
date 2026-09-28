//! Three curves applied in one colour model's canonical channel order.

use serde::{Deserialize, Serialize};

use crate::spec::contract::error::DitheretteError;

use super::{
    chain::{Effect, EffectContext},
    curves::{Curves, Spline},
    image::EffectImage,
    model::ColourModel,
};

const IDENTITY: [[f32; 2]; 2] = [[0.0, 0.0], [1.0, 1.0]];

/// Models available to `model-curves`. Encoded sRGB remains the `curves` effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelCurvesModel {
    LinearRgb,
    Hsl,
    Hsv,
    Oklab,
    Oklch,
    Cielab,
    Cielch,
    Ycbcr,
}

impl From<ModelCurvesModel> for ColourModel {
    fn from(model: ModelCurvesModel) -> Self {
        match model {
            ModelCurvesModel::LinearRgb => Self::LinearRgb,
            ModelCurvesModel::Hsl => Self::Hsl,
            ModelCurvesModel::Hsv => Self::Hsv,
            ModelCurvesModel::Oklab => Self::Oklab,
            ModelCurvesModel::Oklch => Self::Oklch,
            ModelCurvesModel::Cielab => Self::Cielab,
            ModelCurvesModel::Cielch => Self::Cielch,
            ModelCurvesModel::Ycbcr => Self::Ycbcr,
        }
    }
}

/// Three channel curves in the selected model's documented order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCurves {
    pub model: ModelCurvesModel,
    pub curves: [Vec<[f32; 2]>; 3],
}

impl ModelCurves {
    /// True only for the exact three-curve identity tuple.
    pub fn is_identity(&self) -> bool {
        self.curves
            .iter()
            .all(|points| points.as_slice() == IDENTITY)
    }

    fn map(&self, splines: &[Spline; 3], rgb: [f32; 3]) -> [f32; 3] {
        if self.is_identity() {
            return rgb;
        }
        let model = ColourModel::from(self.model);
        let original = model.to_normalized(rgb);
        let mut adjusted = std::array::from_fn(|channel| splines[channel].eval(original[channel]));
        if let Some(channel) = model.hue_channel() {
            let weight = model.hue_weight(rgb, original);
            let delta = (adjusted[channel] - original[channel] + 0.5).rem_euclid(1.0) - 0.5;
            adjusted[channel] = (original[channel] + weight * delta).rem_euclid(1.0);
        }
        model.from_normalized(adjusted)
    }
}

impl Effect for ModelCurves {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        for (index, points) in self.curves.iter().enumerate() {
            Curves::validate_points(points, &format!("{path}.curves.{index}"))?;
        }
        Ok(())
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        if self.is_identity() {
            return;
        }
        let splines = self.curves.each_ref().map(|points| Spline::new(points));
        for rgb in &mut image.rgb {
            *rgb = self.map(&splines, *rgb);
        }
    }
}
