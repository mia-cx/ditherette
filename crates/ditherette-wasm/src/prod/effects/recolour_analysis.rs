//! Deterministic analysis: derive a recolouring recipe from an image and a palette.
//!
//! The palette's reach is its convex hull in the working space: every colour a
//! dithered mixture of palette entries can average to. Analysis fits the
//! image's tone range, overall saturation, and each hue's saturation to that
//! reach, instead of pulling pixels toward individual palette entries.

use crate::prod::contract::request::WorkingSpace;

use super::{
    chain::EffectContext,
    curves::MIN_GAP,
    image::EffectImage,
    recolour::{window, Group, RecolourRecipe, NEUTRAL_CHROMA},
    space::to_opponent,
};

/// Most pixels analysis reads. Larger images are read on a regular grid.
pub const MAX_SAMPLES: u64 = 1 << 18;
/// Hue sectors analysed, evenly spaced from 0 degrees, each `SECTOR_WIDTH` wide at half height.
pub const SECTORS: usize = 6;
pub const SECTOR_WIDTH: f32 = 60.0;
/// Tone quantiles mapped onto the palette's lightness range. The ends ignore 1% outliers.
const TONE_QUANTILES: [f32; 5] = [0.01, 0.25, 0.5, 0.75, 0.99];
/// Share of each tone target taken from the palette's own lightness distribution.
const PALETTE_TONE_SHARE: f32 = 0.5;
/// Palettes with this many lightness levels or fewer take the full share; richer ones less.
const SPARSE_LEVELS: f32 = 4.0;
/// Overall saturation bounds, and per-hue bounds relative to it.
const MAX_CHROMA: f32 = 1.15;
const GROUP_CHROMA: (f32, f32) = (0.25, 1.1);
/// Largest reach-over-chroma ratio counted, so a vivid palette cannot oversaturate a muted image.
const MAX_FIT: f32 = 1.25;
/// A hue sector below this share of coloured image mass gets no group.
const MIN_SECTOR_MASS: f32 = 0.02;
/// A direction counts as reachable when the palette's hull extends this far relative to the image.
const REACH_SHARE: f32 = 0.5;
/// Largest hue turn toward a reachable direction, searched in 5-degree steps.
const MAX_TURN: f32 = 45.0;
/// The shift moves image colours this share of the way toward the palette centroid.
const SHIFT_SHARE: f32 = 0.5;
const MAX_SHIFT_LENGTH: f32 = 0.1;
/// Palette colours this close to neutral count as exactly neutral.
const PALETTE_NEUTRAL: f32 = 0.001;

/// Upper bound on analysis working memory: samples, the sorted lightness copy, and sector inputs.
pub const ANALYSIS_BYTES: u64 = MAX_SAMPLES
    * (std::mem::size_of::<Sample>()
        + std::mem::size_of::<(f32, f32)>()
        + std::mem::size_of::<(f32, f32, f32, f32)>()) as u64;

/// One read pixel: working-space coordinates and its alpha weight in `(0, 1]`.
struct Sample {
    opponent: [f32; 3],
    weight: f32,
}

/// Derives a recipe for `image` in the context's space. Context palette and space are validated.
/// Returns the identity recipe when there is nothing to fit: no visible pixel or palette colour.
pub fn analyze(image: &EffectImage, context: &EffectContext<'_>) -> RecolourRecipe {
    let space = context
        .space
        .expect("recolour analysis requires a working space");
    let mut colors: Vec<[u8; 3]> = Vec::new();
    for color in context.colors() {
        if !colors.contains(&color) {
            colors.push(color);
        }
    }
    let palette: Vec<[f32; 3]> = colors
        .iter()
        .map(|rgb| {
            let [lightness, u, v] = to_opponent(rgb.map(|channel| channel as f32 / 255.0), space);
            // Byte greys carry f32 residue off neutral in perceptual spaces; treat them as grey.
            if u.hypot(v) < PALETTE_NEUTRAL {
                [lightness, 0.0, 0.0]
            } else {
                [lightness, u, v]
            }
        })
        .collect();
    let samples = sample(image, space);
    if samples.is_empty() || palette.is_empty() {
        return RecolourRecipe::identity(space);
    }
    let tone = tone(&samples, &palette);
    let shift = shift(&palette);
    let sectors = sectors(&samples, &palette, shift);
    let chroma = overall_chroma(&sectors);
    let groups = groups(&sectors, &palette, chroma);
    RecolourRecipe {
        space,
        tone,
        chroma,
        shift,
        groups,
    }
}

/// Visible pixels on the coarsest grid step `s` with `ceil(w/s) * ceil(h/s) <= MAX_SAMPLES`,
/// in row-major order. Zero-alpha pixels are skipped; others weigh `alpha / 255`.
fn sample(image: &EffectImage, space: WorkingSpace) -> Vec<Sample> {
    let width = image.dimensions.width_usize();
    let height = image.dimensions.height() as usize;
    let step = sample_step(width, height);
    let mut samples = Vec::new();
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let index = y * width + x;
            let alpha = image.alpha[index];
            if alpha > 0 {
                samples.push(Sample {
                    opponent: to_opponent(image.rgb[index], space),
                    weight: alpha as f32 / 255.0,
                });
            }
        }
    }
    samples
}

/// The coarsest grid step with at most `MAX_SAMPLES` samples. The analysis cache hashes this grid.
pub fn sample_step(width: usize, height: usize) -> usize {
    let mut step = 1;
    while (width.div_ceil(step) * height.div_ceil(step)) as u64 > MAX_SAMPLES {
        step += 1;
    }
    step
}

/// Lightness at weighted quantile `p`: samples sorted by lightness, the first whose
/// cumulative weight reaches `p` of the total.
fn quantile(sorted: &[(f32, f32)], total: f32, p: f32) -> f32 {
    let target = p * total;
    let mut cumulative = 0.0;
    for &(lightness, weight) in sorted {
        cumulative += weight;
        if cumulative >= target {
            return lightness;
        }
    }
    sorted.last().expect("samples are nonempty").0
}

/// Linear interpolation across sorted distinct palette lightness values at rank `p * (n - 1)`.
fn palette_quantile(levels: &[f32], p: f32) -> f32 {
    let position = p * (levels.len() - 1) as f32;
    let below = position.floor() as usize;
    let above = (below + 1).min(levels.len() - 1);
    levels[below] + (position - below as f32) * (levels[above] - levels[below])
}

/// Fits the image's 1%–99% tone range into the palette's lightness range, compressing only
/// what falls outside it. Sparse palettes also pull the quartiles toward their own levels, so
/// detail lands on lightness steps the palette has. A curve within 1e-4 of identity is identity.
fn tone(samples: &[Sample], palette: &[[f32; 3]]) -> Vec<[f32; 2]> {
    let identity = vec![[0.0, 0.0], [1.0, 1.0]];
    let mut sorted: Vec<(f32, f32)> = samples
        .iter()
        .map(|sample| (sample.opponent[0], sample.weight))
        .collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: f32 = sorted.iter().map(|(_, weight)| weight).sum();
    let xs = TONE_QUANTILES.map(|p| quantile(&sorted, total, p).clamp(0.0, 1.0));
    let mut levels: Vec<f32> = palette
        .iter()
        .map(|color| color[0].clamp(0.0, 1.0))
        .collect();
    levels.sort_by(f32::total_cmp);
    levels.dedup();
    let (low, high) = (xs[0], xs[4]);
    let (palette_low, palette_high) = (levels[0], levels[levels.len() - 1]);
    if high - low < MIN_GAP || palette_high - palette_low < MIN_GAP {
        return identity;
    }
    let (mut target_low, mut target_high) = (
        low.clamp(palette_low, palette_high),
        high.clamp(palette_low, palette_high),
    );
    if target_high - target_low < MIN_GAP {
        (target_low, target_high) = (palette_low, palette_high);
    }
    let linear = |x: f32| {
        (target_low + (x - low) / (high - low) * (target_high - target_low))
            .clamp(palette_low, palette_high)
    };
    let share = PALETTE_TONE_SHARE * (SPARSE_LEVELS / levels.len() as f32).min(1.0);
    let ranks = [0.0, 0.25, 0.5, 0.75, 1.0];
    let mut points = vec![[0.0, linear(0.0)]];
    points.extend(xs.iter().zip(ranks).map(|(&x, rank)| {
        let fitted = linear(x);
        [
            x,
            fitted + share * (palette_quantile(&levels, rank) - fitted),
        ]
    }));
    points.push([1.0, linear(1.0)]);
    let mut curve: Vec<[f32; 2]> = Vec::new();
    for [x, y] in points {
        match curve.last() {
            Some(&[last_x, last_y]) if x - last_x >= MIN_GAP => {
                curve.push([x, y.max(last_y).clamp(0.0, 1.0)])
            }
            Some(_) => {}
            None => curve.push([x, y.clamp(0.0, 1.0)]),
        }
    }
    if curve.last().is_some_and(|&[x, _]| x < 1.0) {
        // The 99% quantile sat within one gap of 1; extend flat so the curve spans [0, 1].
        let &[_, y] = curve.last().unwrap();
        curve.pop();
        curve.push([1.0, y]);
    }
    if curve.iter().all(|[x, y]| (x - y).abs() < 1e-4) {
        return identity;
    }
    curve
}

/// How far the palette's convex hull reaches from neutral in hue direction `degrees`.
/// Negative when every palette colour lies on the other side of neutral.
fn reach(palette: &[[f32; 3]], degrees: f32) -> f32 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    palette
        .iter()
        .map(|color| color[1] * cos + color[2] * sin)
        .fold(f32::NEG_INFINITY, f32::max)
}

/// Zero when neutral is inside the palette hull (checked every 5 degrees). Otherwise greys cannot
/// be mixed, so colours move part of the way toward the palette centroid, at most `MAX_SHIFT_LENGTH`.
fn shift(palette: &[[f32; 3]]) -> [f32; 2] {
    if (0..72).all(|step| reach(palette, step as f32 * 5.0) >= 0.0) {
        return [0.0, 0.0];
    }
    let count = palette.len() as f32;
    let centroid = [
        palette.iter().map(|color| color[1]).sum::<f32>() / count,
        palette.iter().map(|color| color[2]).sum::<f32>() / count,
    ];
    let length = centroid[0].hypot(centroid[1]) * SHIFT_SHARE;
    let scale = SHIFT_SHARE * (MAX_SHIFT_LENGTH / length).min(1.0);
    centroid.map(|axis| axis * scale)
}

/// Coloured image mass and mean chroma per hue sector, after `shift`, with the palette's reach.
struct Sector {
    hue: f32,
    mass: f32,
    chroma: f32,
    reach: f32,
}

fn sectors(samples: &[Sample], palette: &[[f32; 3]], shift: [f32; 2]) -> Vec<Sector> {
    // Each sample's hue, chroma, and ramp are the same for every sector, so compute them once.
    // The weight product keeps the reference's order: window, then ramp, then alpha weight.
    let coloured: Vec<(f32, f32, f32, f32)> = samples
        .iter()
        .filter_map(|sample| {
            let u = sample.opponent[1] + shift[0];
            let v = sample.opponent[2] + shift[1];
            let chroma = u.hypot(v);
            let ramp = (chroma / NEUTRAL_CHROMA).min(1.0);
            (ramp != 0.0).then(|| {
                let hue = v.atan2(u).to_degrees().rem_euclid(360.0);
                (hue, ramp, sample.weight, chroma)
            })
        })
        .collect();
    (0..SECTORS)
        .map(|index| {
            let hue = index as f32 * (360.0 / SECTORS as f32);
            let group = Group {
                hue,
                width: SECTOR_WIDTH,
                turn: 0.0,
                chroma: 1.0,
            };
            let (mut mass, mut weighted_chroma) = (0.0, 0.0);
            for &(sample_hue, ramp, sample_weight, chroma) in &coloured {
                let weight = window(sample_hue, &group) * ramp * sample_weight;
                mass += weight;
                weighted_chroma += weight * chroma;
            }
            Sector {
                hue,
                mass,
                chroma: if mass > 0.0 {
                    weighted_chroma / mass
                } else {
                    0.0
                },
                reach: reach(palette, hue),
            }
        })
        .collect()
}

/// Reach over image chroma, capped at 1.25, averaged by coloured mass and clamped to `[0, 1.15]`.
/// A grey image keeps 1.
fn overall_chroma(sectors: &[Sector]) -> f32 {
    let mass: f32 = sectors.iter().map(|sector| sector.mass).sum();
    if mass == 0.0 {
        return 1.0;
    }
    let fit: f32 = sectors
        .iter()
        .filter(|sector| sector.mass > 0.0)
        .map(|sector| sector.mass * (sector.reach.max(0.0) / sector.chroma).min(MAX_FIT))
        .sum();
    let chroma = (fit / mass).clamp(0.0, MAX_CHROMA);
    // Within 2% of neutral is no change worth a round trip.
    if (chroma - 1.0).abs() <= 0.02 {
        1.0
    } else {
        chroma
    }
}

/// One group per sector holding enough coloured mass. A sector the palette cannot reach turns
/// toward the nearest reachable direction within 45 degrees (positive first on ties); its chroma
/// then fits the reach there, relative to the overall scale. Neutral groups are omitted.
fn groups(sectors: &[Sector], palette: &[[f32; 3]], chroma: f32) -> Vec<Group> {
    let mass: f32 = sectors.iter().map(|sector| sector.mass).sum();
    if mass == 0.0 || chroma == 0.0 {
        return Vec::new();
    }
    let mut groups = Vec::new();
    for sector in sectors {
        if sector.mass / mass < MIN_SECTOR_MASS {
            continue;
        }
        let needed = REACH_SHARE * sector.chroma * chroma;
        let reachable = |turn: f32| reach(palette, sector.hue + turn) >= needed;
        let turn = if reachable(0.0) {
            0.0
        } else {
            (1..=(MAX_TURN / 5.0) as usize)
                .flat_map(|step| [step as f32 * 5.0, -(step as f32) * 5.0])
                .find(|&turn| reachable(turn))
                .unwrap_or(0.0)
        };
        let fit = (reach(palette, sector.hue + turn).max(0.0) / sector.chroma).min(MAX_FIT);
        let group_chroma = (fit / chroma).clamp(GROUP_CHROMA.0, GROUP_CHROMA.1);
        if turn != 0.0 || (group_chroma - 1.0).abs() > 0.02 {
            groups.push(Group {
                hue: sector.hue,
                width: SECTOR_WIDTH,
                turn,
                chroma: group_chroma,
            });
        }
    }
    groups
}
