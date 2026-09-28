//! One colour-model channel used as a curve input to adjust another channel.

use serde::{Deserialize, Serialize};

use crate::spec::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::{Effect, EffectContext},
    curves::{Curves, Spline},
    image::EffectImage,
    model::ColourModel,
};

/// Every channel name used by the supported colour models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelChannel {
    Red,
    Green,
    Blue,
    Hue,
    Saturation,
    Lightness,
    Value,
    A,
    B,
    Chroma,
    Luma,
    Cb,
    Cr,
}

/// A named channel in one colour model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColourChannel {
    pub model: ColourModel,
    pub channel: ModelChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Hue,
    Chroma,
    Other,
}

/// Model metadata resolved after validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedChannel {
    pub model: ColourModel,
    pub index: usize,
    pub kind: ChannelKind,
}

impl ColourChannel {
    pub fn resolve(self) -> Option<ResolvedChannel> {
        use ColourModel as M;
        use ModelChannel as C;
        let (index, kind) = match (self.model, self.channel) {
            (M::Srgb | M::LinearRgb, C::Red) => (0, ChannelKind::Other),
            (M::Srgb | M::LinearRgb, C::Green) => (1, ChannelKind::Other),
            (M::Srgb | M::LinearRgb, C::Blue) => (2, ChannelKind::Other),
            (M::Hsl | M::Hsv, C::Hue) => (0, ChannelKind::Hue),
            (M::Hsl | M::Hsv, C::Saturation) => (1, ChannelKind::Chroma),
            (M::Hsl, C::Lightness) => (2, ChannelKind::Other),
            (M::Hsv, C::Value) => (2, ChannelKind::Other),
            (M::Oklab | M::Cielab, C::Lightness) => (0, ChannelKind::Other),
            (M::Oklab | M::Cielab, C::A) => (1, ChannelKind::Other),
            (M::Oklab | M::Cielab, C::B) => (2, ChannelKind::Other),
            (M::Oklch | M::Cielch, C::Lightness) => (0, ChannelKind::Other),
            (M::Oklch | M::Cielch, C::Chroma) => (1, ChannelKind::Chroma),
            (M::Oklch | M::Cielch, C::Hue) => (2, ChannelKind::Hue),
            (M::Ycbcr, C::Luma) => (0, ChannelKind::Other),
            (M::Ycbcr, C::Cb) => (1, ChannelKind::Other),
            (M::Ycbcr, C::Cr) => (2, ChannelKind::Other),
            _ => return None,
        };
        Some(ResolvedChannel {
            model: self.model,
            index,
            kind,
        })
    }
}

/// Cyclic Fritsch-Butland spline for a hue input axis.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicSpline {
    points: Vec<[f32; 2]>,
    tangents: Vec<f32>,
}

impl PeriodicSpline {
    /// Points must start at zero, end at one, and repeat the seam y value.
    pub fn new(points: &[[f32; 2]]) -> Self {
        let last = points.len() - 1;
        let secant =
            |k: usize| (points[k + 1][1] - points[k][1]) / (points[k + 1][0] - points[k][0]);
        let seam = tangent(
            secant(last - 1),
            secant(0),
            points[last][0] - points[last - 1][0],
            points[1][0] - points[0][0],
        );
        let mut tangents = Vec::with_capacity(points.len());
        for k in 0..=last {
            let value = if k == 0 || k == last {
                seam
            } else {
                tangent(
                    secant(k - 1),
                    secant(k),
                    points[k][0] - points[k - 1][0],
                    points[k + 1][0] - points[k][0],
                )
            };
            tangents.push(value);
        }
        Self {
            points: points.to_vec(),
            tangents,
        }
    }

    /// Wraps x at the hue seam, then evaluates the containing Hermite segment.
    pub fn eval(&self, x: f32) -> f32 {
        let x = x.rem_euclid(1.0);
        let last = self.points.len() - 1;
        let k = (0..last)
            .find(|&k| x < self.points[k + 1][0])
            .unwrap_or(last - 1);
        hermite(&self.points, &self.tangents, k, x)
    }
}

fn tangent(before: f32, after: f32, h_before: f32, h_after: f32) -> f32 {
    if before * after <= 0.0 {
        return 0.0;
    }
    let w_before = 2.0 * h_after + h_before;
    let w_after = h_after + 2.0 * h_before;
    (w_before + w_after) / (w_before / before + w_after / after)
}

fn hermite(points: &[[f32; 2]], tangents: &[f32], k: usize, x: f32) -> f32 {
    let [x0, y0] = points[k];
    let [x1, y1] = points[k + 1];
    let (h, secant) = (x1 - x0, (y1 - y0) / (x1 - x0));
    let (m0, m1) = (tangents[k], tangents[k + 1]);
    let c2 = (3.0 * secant - 2.0 * m0 - m1) / h;
    let c3 = (m0 + m1 - 2.0 * secant) / (h * h);
    let s = x - x0;
    y0 + s * (m0 + s * (c2 + s * c3))
}

enum ChannelSpline {
    Linear(Spline),
    Periodic(PeriodicSpline),
}

impl ChannelSpline {
    fn new(points: &[[f32; 2]], periodic: bool) -> Self {
        if periodic {
            Self::Periodic(PeriodicSpline::new(points))
        } else {
            Self::Linear(Spline::new(points))
        }
    }

    fn eval(&self, x: f32) -> f32 {
        match self {
            Self::Linear(spline) => spline.eval(x),
            Self::Periodic(spline) => spline.eval(x),
        }
    }
}

/// One X-channel-to-Y-channel adjustment curve. A y value of 0.5 is neutral.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelCurve {
    pub x: ColourChannel,
    pub y: ColourChannel,
    pub points: Vec<[f32; 2]>,
}

impl ChannelCurve {
    pub fn is_neutral(&self) -> bool {
        self.points.iter().all(|point| point[1] == 0.5)
    }

    fn map(
        &self,
        x: ResolvedChannel,
        y: ResolvedChannel,
        spline: &ChannelSpline,
        rgb: [f32; 3],
    ) -> [f32; 3] {
        if self.is_neutral() {
            return rgb;
        }
        let x_coordinates = x.model.to_normalized(rgb);
        let mut y_coordinates = y.model.to_normalized(rgb);
        let curve = spline.eval(x_coordinates[x.index]);
        let mut weight: f32 = 1.0;
        if x.kind == ChannelKind::Hue {
            weight = weight.min(x.model.hue_weight(rgb, x_coordinates));
        }
        if y.kind == ChannelKind::Hue {
            weight = weight.min(y.model.hue_weight(rgb, y_coordinates));
        }
        match y.kind {
            ChannelKind::Hue => {
                y_coordinates[y.index] =
                    (y_coordinates[y.index] + weight * (curve - 0.5)).rem_euclid(1.0);
            }
            ChannelKind::Chroma => {
                y_coordinates[y.index] *= 1.0 + weight * (2.0 * curve - 1.0);
            }
            ChannelKind::Other => {
                y_coordinates[y.index] += weight * (curve - 0.5);
            }
        }
        y.model.from_normalized(y_coordinates)
    }

    fn validate_channel(
        channel: ColourChannel,
        path: &str,
    ) -> Result<ResolvedChannel, DitheretteError> {
        channel.resolve().ok_or_else(|| {
            DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.channel"),
                "Channel does not belong to the selected colour model.",
            )
        })
    }

    fn validate_periodic_points(&self, path: &str) -> Result<(), DitheretteError> {
        let last = self.points.len() - 1;
        if self.points[0][0] != 0.0 {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.0.0"),
                "A hue-input curve must start at x = 0.",
            ));
        }
        if self.points[last][0] != 1.0 {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.{last}.0"),
                "A hue-input curve must end at x = 1.",
            ));
        }
        if self.points[last][1] != self.points[0][1] {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.{last}.1"),
                "A hue-input curve must repeat its first y value at x = 1.",
            ));
        }
        Ok(())
    }
}

impl Effect for ChannelCurve {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        let x = Self::validate_channel(self.x, &format!("{path}.x"))?;
        Self::validate_channel(self.y, &format!("{path}.y"))?;
        let points_path = format!("{path}.points");
        Curves::validate_points(&self.points, &points_path)?;
        if x.kind == ChannelKind::Hue {
            self.validate_periodic_points(&points_path)?;
        }
        Ok(())
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        if self.is_neutral() {
            return;
        }
        let x = self.x.resolve().expect("validated channel-curve x");
        let y = self.y.resolve().expect("validated channel-curve y");
        let spline = ChannelSpline::new(&self.points, x.kind == ChannelKind::Hue);
        for rgb in &mut image.rgb {
            *rgb = self.map(x, y, &spline, *rgb);
        }
    }
}
