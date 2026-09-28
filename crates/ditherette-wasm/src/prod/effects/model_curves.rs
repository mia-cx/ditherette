//! Three curves applied in one colour model's canonical channel order.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::DitheretteError;

use super::{
    chain::{Effect, EffectContext, PreparedPointwise, StackPath},
    curves::{Curves, Spline},
    image::EffectImage,
    model::ColourModel,
};

const IDENTITY: [[f32; 2]; 2] = [[0.0, 0.0], [1.0, 1.0]];

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCurves {
    pub model: ModelCurvesModel,
    pub curves: [Vec<[f32; 2]>; 3],
}

impl ModelCurves {
    pub fn is_identity(&self) -> bool {
        self.curves
            .iter()
            .all(|points| points.as_slice() == IDENTITY)
    }

    pub fn map(&self, splines: &[Spline; 3], rgb: [f32; 3]) -> [f32; 3] {
        if self.is_identity() {
            return rgb;
        }
        let model = ColourModel::from(self.model);
        let original = model.to_normalized(rgb);
        self.map_coordinates(model, splines, rgb, original)
    }

    fn map_coordinates(
        &self,
        model: ColourModel,
        splines: &[Spline; 3],
        rgb: [f32; 3],
        original: [f32; 3],
    ) -> [f32; 3] {
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
            Curves::validate_points(
                points,
                StackPath::new(format_args!("{path}.curves.{index}")).as_str(),
            )?;
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

    fn per_channel(&self) -> bool {
        self.model == ModelCurvesModel::LinearRgb
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        if self.is_identity() {
            return value;
        }
        let linear = super::space::srgb_unit_to_linear(value);
        super::space::linear_to_srgb_unit(Spline::new(&self.curves[channel]).eval(linear))
    }

    fn pointwise(&self) -> bool {
        true
    }

    fn map_pixel(&self, rgb: [f32; 3], _context: &EffectContext<'_>) -> [f32; 3] {
        if self.is_identity() {
            return rgb;
        }
        let splines = self.curves.each_ref().map(|points| Spline::new(points));
        self.map(&splines, rgb)
    }

    fn prepare_pointwise(&self) -> PreparedPointwise {
        if self.is_identity() {
            return PreparedPointwise::Direct;
        }
        PreparedPointwise::ModelCurves {
            model: self.model.into(),
            splines: self.curves.each_ref().map(|points| Spline::new(points)),
        }
    }

    fn map_prepared(
        &self,
        prepared: PreparedPointwise,
        rgb: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        match prepared {
            PreparedPointwise::ModelCurves { model, splines } => {
                self.map_coordinates(model, &splines, rgb, model.to_normalized(rgb))
            }
            PreparedPointwise::Direct => self.map_pixel(rgb, context),
            _ => unreachable!("model-curves received another effect's prepared state"),
        }
    }

    fn map_prepared_linear(
        &self,
        prepared: PreparedPointwise,
        linear: [f32; 3],
        _context: &EffectContext<'_>,
    ) -> [f32; 3] {
        let PreparedPointwise::ModelCurves {
            model: ColourModel::LinearRgb,
            splines,
        } = prepared
        else {
            unreachable!("only prepared linear-rgb model curves request linear input")
        };
        super::space::from_linear(std::array::from_fn(|channel| {
            splines[channel].eval(linear[channel])
        }))
    }
}
