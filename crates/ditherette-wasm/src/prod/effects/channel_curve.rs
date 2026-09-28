//! One colour-model channel used as a curve input to adjust another channel.

use serde::{Deserialize, Serialize};

use crate::prod::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::{Effect, EffectContext, PreparedPointwise, StackPath},
    curves::{Curves, Spline, MAX_POINTS},
    image::EffectImage,
    model::ColourModel,
};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicSpline {
    points: [[f32; 2]; MAX_POINTS],
    tangents: [f32; MAX_POINTS],
    len: usize,
}

impl PeriodicSpline {
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
        let mut tangents = [0.0; MAX_POINTS];
        for (k, value) in tangents.iter_mut().enumerate().take(points.len()) {
            *value = if k == 0 || k == last {
                seam
            } else {
                tangent(
                    secant(k - 1),
                    secant(k),
                    points[k][0] - points[k - 1][0],
                    points[k + 1][0] - points[k][0],
                )
            };
        }
        let mut inline = [[0.0; 2]; MAX_POINTS];
        inline[..points.len()].copy_from_slice(points);
        Self {
            points: inline,
            tangents,
            len: points.len(),
        }
    }

    pub fn eval(&self, x: f32) -> f32 {
        let x = x.rem_euclid(1.0);
        let last = self.len - 1;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChannelSpline {
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

    fn eval(self, x: f32) -> f32 {
        match self {
            Self::Linear(spline) => spline.eval(x),
            Self::Periodic(spline) => spline.eval(x),
        }
    }
}

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

    pub fn map(
        &self,
        x: ResolvedChannel,
        y: ResolvedChannel,
        spline: ChannelSpline,
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
        // No adjustment at all, so skip the Y round-trip, which isn't exact in every model.
        if weight == 0.0 {
            return rgb;
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

    fn resolved(&self) -> (ResolvedChannel, ResolvedChannel) {
        (
            self.x.resolve().expect("validated channel-curve x"),
            self.y.resolve().expect("validated channel-curve y"),
        )
    }

    fn validate_channel(
        channel: ColourChannel,
        path: &str,
    ) -> Result<ResolvedChannel, DitheretteError> {
        channel.resolve().ok_or_else(|| {
            DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.channel")).as_str(),
                "Channel does not belong to the selected colour model.",
            )
        })
    }

    fn validate_periodic_points(&self, path: &str) -> Result<(), DitheretteError> {
        let last = self.points.len() - 1;
        let invalid = |index: usize, coordinate: usize, message| {
            DitheretteError::new(
                ErrorCode::InvalidSettings,
                StackPath::new(format_args!("{path}.{index}.{coordinate}")).as_str(),
                message,
            )
        };
        if self.points[0][0] != 0.0 {
            return Err(invalid(0, 0, "A hue-input curve must start at x = 0."));
        }
        if self.points[last][0] != 1.0 {
            return Err(invalid(last, 0, "A hue-input curve must end at x = 1."));
        }
        if self.points[last][1] != self.points[0][1] {
            return Err(invalid(
                last,
                1,
                "A hue-input curve must repeat its first y value at x = 1.",
            ));
        }
        Ok(())
    }

    fn per_channel_pair(&self) -> Option<(ResolvedChannel, ResolvedChannel)> {
        let (x, y) = self.resolved();
        // A linear-light output would round-trip the untouched channels too, so it isn't per-channel.
        let rgb_input = matches!(x.model, ColourModel::Srgb | ColourModel::LinearRgb);
        (rgb_input && matches!(y.model, ColourModel::Srgb) && x.index == y.index).then_some((x, y))
    }
}

impl Effect for ChannelCurve {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        let x = Self::validate_channel(self.x, StackPath::new(format_args!("{path}.x")).as_str())?;
        Self::validate_channel(self.y, StackPath::new(format_args!("{path}.y")).as_str())?;
        let points_path = StackPath::new(format_args!("{path}.points"));
        Curves::validate_points(&self.points, points_path.as_str())?;
        if x.kind == ChannelKind::Hue {
            self.validate_periodic_points(points_path.as_str())?;
        }
        Ok(())
    }

    fn apply(&self, image: &mut EffectImage, _context: &EffectContext<'_>) {
        if self.is_neutral() {
            return;
        }
        let (x, y) = self.resolved();
        let spline = ChannelSpline::new(&self.points, x.kind == ChannelKind::Hue);
        for rgb in &mut image.rgb {
            *rgb = self.map(x, y, spline, *rgb);
        }
    }

    fn per_channel(&self) -> bool {
        self.per_channel_pair().is_some()
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        if self.is_neutral() {
            return value;
        }
        let (x, y) = self
            .per_channel_pair()
            .expect("only eligible channel curves request a channel map");
        if channel != x.index {
            return value;
        }
        let spline = ChannelSpline::new(&self.points, false);
        self.map(x, y, spline, [value; 3])[channel]
    }

    fn pointwise(&self) -> bool {
        true
    }

    fn map_pixel(&self, rgb: [f32; 3], _context: &EffectContext<'_>) -> [f32; 3] {
        if self.is_neutral() {
            return rgb;
        }
        let (x, y) = self.resolved();
        let spline = ChannelSpline::new(&self.points, x.kind == ChannelKind::Hue);
        self.map(x, y, spline, rgb)
    }

    fn prepare_pointwise(&self) -> PreparedPointwise {
        if self.is_neutral() {
            return PreparedPointwise::Direct;
        }
        let (x, y) = self.resolved();
        PreparedPointwise::ChannelCurve {
            x,
            y,
            spline: ChannelSpline::new(&self.points, x.kind == ChannelKind::Hue),
        }
    }

    fn map_prepared(
        &self,
        prepared: PreparedPointwise,
        rgb: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        match prepared {
            PreparedPointwise::ChannelCurve { x, y, spline } => self.map(x, y, spline, rgb),
            PreparedPointwise::Direct => self.map_pixel(rgb, context),
            _ => unreachable!("channel-curve received another effect's prepared state"),
        }
    }
}
