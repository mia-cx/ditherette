//! Palette-independent axis plans preserve anchors and source halo coordinates.

use crate::{
    image::ImageDimensions,
    prod::{
        contract::request::{Anchor, ResizePolicy, Support},
        resize::{
            common::alignment::{map_axis_position, AxisAlignment},
            scalar::nearest::alignment::map_axis_coordinate,
        },
    },
};

use super::super::Fallback;

#[derive(Clone, Copy)]
pub(super) struct Filter {
    pub kind: u32,
    pub anchor: Anchor,
    pub support: Support,
}
impl Filter {
    pub fn new(policy: ResizePolicy) -> Self {
        match policy {
            ResizePolicy::Nearest { anchor } => Self {
                kind: 0,
                anchor,
                support: Support::Fixed,
            },
            ResizePolicy::Area {} => Self {
                kind: 1,
                anchor: Anchor::Center,
                support: Support::ScaleAware,
            },
            ResizePolicy::Bilinear { anchor } => Self {
                kind: 2,
                anchor,
                support: Support::ScaleAware,
            },
            ResizePolicy::Bicubic { anchor, support } => Self {
                kind: 3,
                anchor,
                support,
            },
            ResizePolicy::Lanczos2 { anchor, support } => Self {
                kind: 4,
                anchor,
                support,
            },
            ResizePolicy::Lanczos3 { anchor, support } => Self {
                kind: 5,
                anchor,
                support,
            },
            ResizePolicy::Trilinear { anchor } => Self {
                kind: 2,
                anchor,
                support: Support::ScaleAware,
            },
        }
    }
    pub fn radius(self) -> f64 {
        match self.kind {
            0 => 0.,
            3 | 4 => 2.,
            5 => 3.,
            _ => 1.,
        }
    }
    fn weight(self, x: f64) -> f64 {
        let x = x.abs();
        match self.kind {
            2 if x < 1. => 1. - x,
            3 if x < 1. => 1.5 * x * x * x - 2.5 * x * x + 1.,
            3 if x < 2. => -0.5 * x * x * x + 2.5 * x * x - 4. * x + 2.,
            4 | 5 if x == 0. => 1.,
            4 | 5 if x < self.radius() => {
                let sinc = |v: f64| {
                    let a = std::f64::consts::PI * v;
                    a.sin() / a
                };
                sinc(x) * sinc(x / self.radius())
            }
            _ => 0.,
        }
    }
    fn scale(self, source: u32, output: u32) -> f64 {
        if self.support == Support::ScaleAware {
            (f64::from(source) / f64::from(output)).max(1.)
        } else {
            1.
        }
    }
}

pub(super) struct Axis {
    pub words: Vec<u32>,
    pub ranges: Vec<(u32, u32)>,
}
impl Axis {
    pub fn required(source: u32, output: u32, filter: Filter) -> u64 {
        let scale = f64::from(source) / f64::from(output);
        let taps = match filter.kind {
            0 => 1,
            1 => scale.ceil() as u64 + 1,
            _ => (2. * filter.radius() * filter.scale(source, output)).ceil() as u64 + 2,
        };
        u64::from(output) * (16 + 8 * taps + 8)
    }
    pub fn new(
        source: u32,
        output: u32,
        alignment: AxisAlignment,
        filter: Filter,
    ) -> Result<Self, Fallback> {
        let capacity = Self::required(source, output, filter);
        let mut words = Vec::new();
        words
            .try_reserve_exact(((capacity - u64::from(output) * 8) / 4) as usize)
            .map_err(|_| Fallback::MemoryLimit)?;
        words.resize(output as usize * 4, 0);
        let mut ranges = Vec::new();
        ranges
            .try_reserve_exact(output as usize)
            .map_err(|_| Fallback::MemoryLimit)?;
        let scale = filter.scale(source, output);
        for coordinate in 0..output {
            let start = words.len() as u32;
            let position = map_axis_position(coordinate, source, output, alignment);
            let mut minimum = source - 1;
            let mut maximum = 0;
            let mut total = 0.;
            let mut push = |index: u32, weight: f64| {
                if weight == 0. {
                    return;
                }
                words.extend([index, (weight as f32).to_bits()]);
                minimum = minimum.min(index);
                maximum = maximum.max(index);
                total += weight;
            };
            match filter.kind {
                0 => push(
                    map_axis_coordinate(coordinate, source, output, alignment),
                    1.,
                ),
                1 => {
                    let scale = f64::from(source) / f64::from(output);
                    let a = f64::from(coordinate) * scale;
                    let b = f64::from(coordinate + 1) * scale;
                    for tap in a.floor() as i64..b.ceil() as i64 {
                        push(
                            tap.clamp(0, i64::from(source) - 1) as u32,
                            (b.min(tap as f64 + 1.) - a.max(tap as f64)).max(0.),
                        );
                    }
                }
                _ => {
                    for tap in (position - filter.radius() * scale).floor() as i64
                        ..=(position + filter.radius() * scale).ceil() as i64
                    {
                        push(
                            tap.clamp(0, i64::from(source) - 1) as u32,
                            filter.weight((tap as f64 - position) / scale),
                        );
                    }
                }
            }
            let end = words.len() as u32;
            for tap in words[start as usize..end as usize].chunks_exact_mut(2) {
                tap[1] = ((f64::from(f32::from_bits(tap[1])) / total) as f32).to_bits();
            }
            let lobe_first = (position - scale).floor() as i64;
            let lobe_last = (position + scale).ceil() as i64;
            let mut first = source - 1;
            let mut last = 0;
            for tap in lobe_first..=lobe_last {
                if ((tap as f64 - position) / scale).abs() < 1. {
                    let i = tap.clamp(0, i64::from(source) - 1) as u32;
                    first = first.min(i);
                    last = last.max(i);
                }
            }
            words[coordinate as usize * 4..coordinate as usize * 4 + 4].copy_from_slice(&[
                start,
                (end - start) / 2,
                first,
                last,
            ]);
            ranges.push((minimum, maximum));
        }
        Ok(Self { words, ranges })
    }
    pub fn rows(&self, start: u32, height: u32) -> (u32, u32) {
        self.ranges[start as usize..(start + height) as usize]
            .iter()
            .fold((u32::MAX, 0), |(lo, hi), &(a, b)| (lo.min(a), hi.max(b)))
    }
}

pub(super) fn axes(anchor: Anchor) -> (AxisAlignment, AxisAlignment) {
    use AxisAlignment::{Center, End, Start};
    match anchor {
        Anchor::TopLeft => (Start, Start),
        Anchor::Top => (Center, Start),
        Anchor::TopRight => (End, Start),
        Anchor::Left => (Start, Center),
        Anchor::Center => (Center, Center),
        Anchor::Right => (End, Center),
        Anchor::BottomLeft => (Start, End),
        Anchor::Bottom => (Center, End),
        Anchor::BottomRight => (End, End),
    }
}
pub(super) fn pixels(dimensions: ImageDimensions) -> u64 {
    u64::from(dimensions.width()) * u64::from(dimensions.height())
}
