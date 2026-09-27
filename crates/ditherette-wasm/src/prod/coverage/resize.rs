//! Exact coverage-weighted resize, independent from the frozen reference tree.

use std::{f64::consts::PI, mem::size_of};

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, LinearRgba32, Rgba8},
    prod::{
        contract::{
            failure::Failure,
            request::{Anchor, ResizePolicy, Support},
        },
        resize::{
            common::{alignment::ResizeAnchor, allocation::CapacityBudget},
            scalar::trilinear::PreparedTrilinear,
        },
    },
};

const CHANNELS: usize = 4;
const BYTE_MAX: f64 = 255.0;

pub(crate) fn required_bytes(
    source: ImageDimensions,
    output: ImageDimensions,
    policy: ResizePolicy,
) -> Result<u64, Failure> {
    let source_len = source
        .storage_len::<LinearRgba32>()
        .map_err(|_| memory_limit())?;
    let output_len = output
        .storage_len::<LinearRgba32>()
        .map_err(|_| memory_limit())?;
    let carriers = source_len
        .checked_add(output_len)
        .ok_or_else(memory_limit)? as u64
        * size_of::<f32>() as u64;
    let prepared = match policy {
        ResizePolicy::Trilinear { .. } => {
            PreparedTrilinear::<LinearRgba32>::required_bytes(source, output)?
        }
        ResizePolicy::Area {} => {
            axis_plan_bytes(output.width(), area_taps(source.width(), output.width()))?
                .checked_add(axis_plan_bytes(
                    output.height(),
                    area_taps(source.height(), output.height()),
                )?)
                .ok_or_else(memory_limit)?
        }
        ResizePolicy::Bilinear { .. } => {
            reconstruction_plan_bytes(source, output, 1.0, Support::ScaleAware)?
        }
        ResizePolicy::Bicubic { support, .. } => {
            reconstruction_plan_bytes(source, output, 2.0, support)?
        }
        ResizePolicy::Lanczos2 { support, .. } => {
            reconstruction_plan_bytes(source, output, 2.0, support)?
        }
        ResizePolicy::Lanczos3 { support, .. } => {
            reconstruction_plan_bytes(source, output, 3.0, support)?
        }
        ResizePolicy::Nearest { .. } => 0,
    };
    carriers.checked_add(prepared).ok_or_else(memory_limit)
}

pub(crate) fn resize(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    policy: ResizePolicy,
    limit: u64,
    progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let required = required_bytes(source_dimensions, output_dimensions, policy)?;
    let mut budget = CapacityBudget::new(limit);
    budget.check_additional(required)?;
    let source_len = source_dimensions
        .storage_len::<LinearRgba32>()
        .map_err(|_| memory_limit())?;
    let output_len = output_dimensions
        .storage_len::<LinearRgba32>()
        .map_err(|_| memory_limit())?;
    let mut premultiplied = budget.vector::<f32>(source_len)?;
    premultiplied.resize(source_len, 0.0);
    let mut filtered = budget.vector::<f32>(output_len)?;
    filtered.resize(output_len, 0.0);

    let filter_rows = filter_rows(source_dimensions, output_dimensions, policy);
    let total = source_dimensions.height() + filter_rows + output_dimensions.height();
    progress(0, total)?;
    let source_width = source_dimensions.width_usize();
    for y in 0..source_dimensions.height() {
        let from = source.row(y).expect("validated source row");
        let start = y as usize * source_width * CHANNELS;
        for (pixel, target) in from
            .chunks_exact(CHANNELS)
            .zip(premultiplied[start..start + source_width * CHANNELS].chunks_exact_mut(CHANNELS))
        {
            let alpha = f64::from(pixel[3]);
            let coverage = alpha / BYTE_MAX;
            target[0] = (f64::from(pixel[0]) * coverage) as f32;
            target[1] = (f64::from(pixel[1]) * coverage) as f32;
            target[2] = (f64::from(pixel[2]) * coverage) as f32;
            target[3] = alpha as f32;
        }
        progress(y + 1, total)?;
    }

    let carrier = ImageView::<LinearRgba32>::packed(&premultiplied, source_dimensions)
        .expect("packed carrier");
    let target = ImageViewMut::<LinearRgba32>::packed(&mut filtered, output_dimensions)
        .expect("packed carrier");
    let offset = source_dimensions.height();
    match policy {
        ResizePolicy::Area {} => {
            filter_area(carrier, target, &mut budget, offset, total, progress)?
        }
        ResizePolicy::Bilinear { anchor } => filter_reconstruction(
            carrier,
            target,
            anchor,
            Support::ScaleAware,
            Kernel::Triangle,
            &mut budget,
            offset,
            total,
            progress,
        )?,
        ResizePolicy::Bicubic { anchor, support } => filter_reconstruction(
            carrier,
            target,
            anchor,
            support,
            Kernel::Cubic,
            &mut budget,
            offset,
            total,
            progress,
        )?,
        ResizePolicy::Lanczos2 { anchor, support } => filter_reconstruction(
            carrier,
            target,
            anchor,
            support,
            Kernel::Lanczos(2),
            &mut budget,
            offset,
            total,
            progress,
        )?,
        ResizePolicy::Lanczos3 { anchor, support } => filter_reconstruction(
            carrier,
            target,
            anchor,
            support,
            Kernel::Lanczos(3),
            &mut budget,
            offset,
            total,
            progress,
        )?,
        ResizePolicy::Trilinear { anchor } => {
            let remaining = limit.checked_sub(budget.used()).ok_or_else(memory_limit)?;
            let mut prepared = PreparedTrilinear::<LinearRgba32>::try_new(
                source_dimensions,
                output_dimensions,
                resize_anchor(anchor),
                remaining,
            )?;
            prepared.execute_with_progress(carrier, target, &mut |done, _| {
                progress(offset + done, total)
            })?;
        }
        ResizePolicy::Nearest { .. } => unreachable!("coverage excludes nearest"),
    }

    let filtered =
        ImageView::<LinearRgba32>::packed(&filtered, output_dimensions).expect("packed carrier");
    let clamp = matches!(
        policy,
        ResizePolicy::Bicubic { .. }
            | ResizePolicy::Lanczos2 { .. }
            | ResizePolicy::Lanczos3 { .. }
    );
    for y in 0..output_dimensions.height() {
        let row = filtered.row(y).expect("validated filtered row");
        let target = output.row_mut(y).expect("validated output row");
        for x in 0..output_dimensions.width() {
            let pixel = &row[x as usize * CHANNELS..][..CHANNELS];
            let out = &mut target[x as usize * CHANNELS..][..CHANNELS];
            let alpha = f64::from(pixel[3]);
            let output_alpha = if clamp {
                let (low, high) = main_lobe_alpha(source, output_dimensions, policy, x, y);
                alpha.clamp(f64::from(low), f64::from(high))
            } else {
                alpha
            };
            out[3] = byte(output_alpha);
            if out[3] == 0 || alpha <= 0.0 {
                out[..3].fill(0);
            } else {
                for channel in 0..3 {
                    out[channel] = byte(f64::from(pixel[channel]) * BYTE_MAX / alpha);
                }
            }
        }
        progress(offset + filter_rows + y + 1, total)?;
    }
    Ok(())
}

fn filter_rows(source: ImageDimensions, output: ImageDimensions, policy: ResizePolicy) -> u32 {
    if matches!(policy, ResizePolicy::Trilinear { .. }) {
        let factor = (f64::from(source.width()) / f64::from(output.width()))
            .max(f64::from(source.height()) / f64::from(output.height()));
        if factor <= 1.0 {
            return output.height();
        }
        let lower = factor.log2().floor() as usize;
        let upper = factor.log2().ceil() as usize;
        let mut dims = source;
        let mut rows = 0;
        for _ in 0..upper {
            if dims.width() == 1 && dims.height() == 1 {
                break;
            }
            dims =
                ImageDimensions::new(dims.width().div_ceil(2), dims.height().div_ceil(2)).unwrap();
            rows += dims.height();
        }
        rows + output.height()
            * if lower == upper || dims.width() == 1 && dims.height() == 1 {
                1
            } else {
                3
            }
    } else {
        output.height()
    }
}

fn filter_area(
    source: ImageView<'_, LinearRgba32>,
    mut output: ImageViewMut<'_, LinearRgba32>,
    budget: &mut CapacityBudget,
    offset: u32,
    total: u32,
    progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let from = source.dimensions();
    let to = output.dimensions();
    let x_scale = f64::from(from.width()) / f64::from(to.width());
    let y_scale = f64::from(from.height()) / f64::from(to.height());
    let area = x_scale * y_scale;
    let x_plan = AxisPlan::area(budget, from.width(), to.width())?;
    let y_plan = AxisPlan::area(budget, from.height(), to.height())?;
    for oy in 0..to.height() {
        let row = output.row_mut(oy).unwrap();
        for ox in 0..to.width() {
            let mut sum = [0.0; CHANNELS];
            for ytap in y_plan.taps(oy) {
                let source_row = source.row(ytap.index).unwrap();
                for xtap in x_plan.taps(ox) {
                    let weight = xtap.weight * ytap.weight / area;
                    let start = xtap.index as usize * CHANNELS;
                    for channel in 0..CHANNELS {
                        sum[channel] += f64::from(source_row[start + channel]) * weight;
                    }
                }
            }
            let target = &mut row[ox as usize * CHANNELS..][..CHANNELS];
            for channel in 0..CHANNELS {
                target[channel] = sum[channel] as f32;
            }
        }
        progress(offset + oy + 1, total)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Kernel {
    Triangle,
    Cubic,
    Lanczos(u32),
}

impl Kernel {
    fn radius(self) -> f64 {
        match self {
            Self::Triangle => 1.0,
            Self::Cubic => 2.0,
            Self::Lanczos(radius) => f64::from(radius),
        }
    }
    fn weight(self, distance: f64) -> f64 {
        let x = distance.abs();
        match self {
            Self::Triangle => {
                if x < 1.0 {
                    1.0 - x
                } else {
                    0.0
                }
            }
            Self::Cubic => {
                if x < 1.0 {
                    1.5 * x * x * x - 2.5 * x * x + 1.0
                } else if x < 2.0 {
                    -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0
                } else {
                    0.0
                }
            }
            Self::Lanczos(_) if x == 0.0 => 1.0,
            Self::Lanczos(radius) if x < f64::from(radius) => sinc(x) * sinc(x / f64::from(radius)),
            Self::Lanczos(_) => 0.0,
        }
    }
}

fn filter_reconstruction(
    source: ImageView<'_, LinearRgba32>,
    mut output: ImageViewMut<'_, LinearRgba32>,
    anchor: Anchor,
    support: Support,
    kernel: Kernel,
    budget: &mut CapacityBudget,
    offset: u32,
    total: u32,
    progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let from = source.dimensions();
    let to = output.dimensions();
    let (xa, ya) = resize_anchor(anchor).axes();
    let xs = axis_scale(from.width(), to.width(), support);
    let ys = axis_scale(from.height(), to.height(), support);
    let x_plan = AxisPlan::reconstruction(budget, from.width(), to.width(), xa, xs, kernel)?;
    let y_plan = AxisPlan::reconstruction(budget, from.height(), to.height(), ya, ys, kernel)?;
    for oy in 0..to.height() {
        let row = output.row_mut(oy).unwrap();
        for ox in 0..to.width() {
            let mut sum = [0.0; CHANNELS];
            let mut total_weight = 0.0;
            for ytap in y_plan.taps(oy) {
                let source_row = source.row(ytap.index).unwrap();
                for xtap in x_plan.taps(ox) {
                    let weight = xtap.weight * ytap.weight;
                    let start = xtap.index as usize * CHANNELS;
                    total_weight += weight;
                    for channel in 0..CHANNELS {
                        sum[channel] += f64::from(source_row[start + channel]) * weight;
                    }
                }
            }
            let target = &mut row[ox as usize * CHANNELS..][..CHANNELS];
            for channel in 0..CHANNELS {
                target[channel] = (sum[channel] / total_weight) as f32;
            }
        }
        progress(offset + oy + 1, total)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct AxisTap {
    index: u32,
    weight: f64,
}

struct AxisPlan {
    starts: Vec<u32>,
    taps: Vec<AxisTap>,
}

impl AxisPlan {
    fn reserve(budget: &mut CapacityBudget, outputs: u32, taps: usize) -> Result<Self, Failure> {
        Ok(Self {
            starts: budget.vector(outputs as usize + 1)?,
            taps: budget.vector(taps)?,
        })
    }

    fn area(budget: &mut CapacityBudget, source: u32, output: u32) -> Result<Self, Failure> {
        let per_output = area_taps(source, output);
        let mut plan = Self::reserve(budget, output, output as usize * per_output)?;
        let scale = f64::from(source) / f64::from(output);
        plan.starts.push(0);
        for coordinate in 0..output {
            let start = f64::from(coordinate) * scale;
            let end = f64::from(coordinate + 1) * scale;
            for tap in start.floor() as i64..end.ceil() as i64 {
                let weight = overlap(start, end, tap as f64, tap as f64 + 1.0);
                if weight != 0.0 {
                    plan.taps.push(AxisTap {
                        index: tap.clamp(0, i64::from(source) - 1) as u32,
                        weight,
                    });
                }
            }
            plan.starts.push(plan.taps.len() as u32);
        }
        Ok(plan)
    }

    fn reconstruction(
        budget: &mut CapacityBudget,
        source: u32,
        output: u32,
        alignment: crate::prod::resize::common::alignment::AxisAlignment,
        scale: f64,
        kernel: Kernel,
    ) -> Result<Self, Failure> {
        let per_output = reconstruction_taps(kernel.radius() * scale);
        let mut plan = Self::reserve(budget, output, output as usize * per_output)?;
        plan.starts.push(0);
        for coordinate in 0..output {
            let position = crate::prod::resize::common::alignment::map_axis_position(
                coordinate, source, output, alignment,
            );
            for tap in support_range(position, kernel.radius() * scale) {
                let weight = kernel.weight((tap as f64 - position) / scale);
                if weight != 0.0 {
                    plan.taps.push(AxisTap {
                        index: tap.clamp(0, i64::from(source) - 1) as u32,
                        weight,
                    });
                }
            }
            plan.starts.push(plan.taps.len() as u32);
        }
        Ok(plan)
    }

    fn taps(&self, output: u32) -> &[AxisTap] {
        &self.taps[self.starts[output as usize] as usize..self.starts[output as usize + 1] as usize]
    }
}

fn area_taps(source: u32, output: u32) -> usize {
    (f64::from(source) / f64::from(output)).ceil() as usize + 1
}

fn reconstruction_taps(support: f64) -> usize {
    (support * 2.0).ceil() as usize + 2
}

fn axis_plan_bytes(outputs: u32, taps: usize) -> Result<u64, Failure> {
    (u64::from(outputs) + 1)
        .checked_mul(size_of::<u32>() as u64)
        .and_then(|bytes| {
            bytes.checked_add(u64::from(outputs) * taps as u64 * size_of::<AxisTap>() as u64)
        })
        .ok_or_else(memory_limit)
}

fn reconstruction_plan_bytes(
    source: ImageDimensions,
    output: ImageDimensions,
    radius: f64,
    support: Support,
) -> Result<u64, Failure> {
    let x = reconstruction_taps(radius * axis_scale(source.width(), output.width(), support));
    let y = reconstruction_taps(radius * axis_scale(source.height(), output.height(), support));
    axis_plan_bytes(output.width(), x)?
        .checked_add(axis_plan_bytes(output.height(), y)?)
        .ok_or_else(memory_limit)
}

fn main_lobe_alpha(
    source: ImageView<'_, Rgba8>,
    output: ImageDimensions,
    policy: ResizePolicy,
    x: u32,
    y: u32,
) -> (u8, u8) {
    let (anchor, support) = match policy {
        ResizePolicy::Bicubic { anchor, support }
        | ResizePolicy::Lanczos2 { anchor, support }
        | ResizePolicy::Lanczos3 { anchor, support } => (anchor, support),
        _ => unreachable!(),
    };
    let dims = source.dimensions();
    let (xa, ya) = resize_anchor(anchor).axes();
    let xp = crate::prod::resize::common::alignment::map_axis_position(
        x,
        dims.width(),
        output.width(),
        xa,
    );
    let yp = crate::prod::resize::common::alignment::map_axis_position(
        y,
        dims.height(),
        output.height(),
        ya,
    );
    let xs = axis_scale(dims.width(), output.width(), support);
    let ys = axis_scale(dims.height(), output.height(), support);
    let mut range = (u8::MAX, u8::MIN);
    for sy in lobe_taps(yp, ys, dims.height()) {
        let row = source.row(sy).unwrap();
        for sx in lobe_taps(xp, xs, dims.width()) {
            let alpha = row[sx as usize * CHANNELS + 3];
            range = (range.0.min(alpha), range.1.max(alpha));
        }
    }
    range
}

fn lobe_taps(position: f64, scale: f64, len: u32) -> impl Iterator<Item = u32> {
    let first = (position - scale).floor() as i64;
    let last = (position + scale).ceil() as i64;
    (first..=last)
        .filter(move |&tap| ((tap as f64 - position) / scale).abs() < 1.0)
        .map(move |tap| tap.clamp(0, i64::from(len) - 1) as u32)
}

fn resize_anchor(anchor: Anchor) -> ResizeAnchor {
    match anchor {
        Anchor::TopLeft => ResizeAnchor::TopLeft,
        Anchor::Top => ResizeAnchor::Top,
        Anchor::TopRight => ResizeAnchor::TopRight,
        Anchor::Left => ResizeAnchor::Left,
        Anchor::Center => ResizeAnchor::Center,
        Anchor::Right => ResizeAnchor::Right,
        Anchor::BottomLeft => ResizeAnchor::BottomLeft,
        Anchor::Bottom => ResizeAnchor::Bottom,
        Anchor::BottomRight => ResizeAnchor::BottomRight,
    }
}

fn axis_scale(source: u32, output: u32, support: Support) -> f64 {
    match support {
        Support::Fixed => 1.0,
        Support::ScaleAware => (f64::from(source) / f64::from(output)).max(1.0),
    }
}
fn support_range(position: f64, support: f64) -> std::ops::RangeInclusive<i64> {
    (position - support).floor() as i64..=(position + support).ceil() as i64
}
fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}
fn sinc(value: f64) -> f64 {
    if value == 0.0 {
        1.0
    } else {
        let x = PI * value;
        x.sin() / x
    }
}
fn byte(value: f64) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}
fn memory_limit() -> Failure {
    Failure::new(
        crate::prod::contract::error::ErrorCode::MemoryLimit,
        crate::prod::contract::failure::ErrorPath::MemoryLimitBytes,
    )
}
