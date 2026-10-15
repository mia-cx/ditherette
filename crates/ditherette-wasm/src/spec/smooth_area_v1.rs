//! Overlapping square-area contrast placement. See `smooth_area_v1.md`.

pub mod render;

use crate::image::{ImageDimensions, ImageView, Rgba8};
use crate::spec::color::oklab::rgb8_to_oklab;

/// Pixel records plus three source colour rows, reused as one line queue.
pub fn scratch_len(dimensions: ImageDimensions) -> usize {
    dimensions.pixel_count().expect("validated dimensions")
        + (3 * dimensions.width_usize()).max(dimensions.height_usize())
}

/// Selects complete squares bounded by fine contrast and scale-dependent Oklab residual.
/// Total colour change across a smooth ramp is unrestricted. Flat plateaus stay included.
pub fn select<E>(
    source: ImageView<'_, Rgba8>,
    sample_size: u32,
    contrast_threshold: f32,
    scratch: &mut [[f32; 3]],
    mut check: impl FnMut() -> Result<(), E>,
) -> Result<(), E> {
    let dimensions = source.dimensions();
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    assert_eq!(scratch.len(), scratch_len(dimensions));
    let (cells, queue) = scratch.split_at_mut(width * height);
    cells.fill([0.0; 3]);
    let sample_width = (sample_size as usize).min(width);
    let sample_height = (sample_size as usize).min(height);
    let columns = width - sample_width + 1;
    let rows = height - sample_height + 1;
    let radius = sample_size as usize / 2;
    for y in 0..height {
        let row = source.row(y as u32).expect("validated row");
        for (pixel, target) in row
            .chunks_exact(4)
            .zip(&mut cells[y * width..(y + 1) * width])
        {
            *target = rgb8_to_oklab([pixel[0], pixel[1], pixel[2]]);
        }
        check()?;
    }
    // The coarse colour is a symmetric 2D box mean. A compact transition differs
    // from its coarse colour; a broad locally linear ramp largely cancels out.
    for y in 0..height {
        average_line(dimensions, false, y, radius, cells, queue);
        check()?;
    }
    for x in 0..width {
        average_line(dimensions, true, x, radius, cells, queue);
        check()?;
    }
    // The one-pixel read halo makes a strong edge fail every containing footprint,
    // including squares ending immediately beside it. It never grows with sample size.
    let mut next_row = 0;
    for y in 0..height {
        // Reconvert each original row once, so coarse means need no second image plane.
        while next_row <= (y + 1).min(height - 1) {
            let row = source.row(next_row as u32).expect("validated row");
            let slot = next_row % 3;
            for (pixel, target) in row
                .chunks_exact(4)
                .zip(&mut queue[slot * width..(slot + 1) * width])
            {
                *target = rgb8_to_oklab([pixel[0], pixel[1], pixel[2]]);
            }
            next_row += 1;
            check()?;
        }
        for x in 0..width {
            let center = queue[(y % 3) * width + x];
            // Twice the residual uses the full jump magnitude at a symmetric step.
            let mut contrast = (2.0 * color_distance(center, cells[y * width + x])).min(1.0);
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                let row = &queue[(ny % 3) * width..(ny % 3 + 1) * width];
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    contrast = contrast.max(color_distance(center, row[nx]).min(1.0));
                }
            }
            cells[y * width + x][2] = contrast;
        }
        check()?;
    }
    for y in 0..height {
        maximum_line(dimensions, false, y, sample_width, cells, queue);
        check()?;
    }
    for x in 0..columns {
        maximum_line(dimensions, true, x, sample_height, cells, queue);
        check()?;
    }
    // Dilate anchors horizontally, then vertically. Complete interval unions
    // need no overlap counter, so large images cannot overflow coverage counts.
    for y in 0..height {
        let mut right = 0;
        for x in 0..width {
            let cell = &mut cells[y * width + x];
            if y < rows && x < columns && cell[2] <= contrast_threshold {
                right = x + sample_width;
            }
            cell[0] = if x < right { 1.0 } else { 0.0 };
        }
        check()?;
    }
    for x in 0..width {
        let mut bottom = 0;
        for y in 0..height {
            let cell = &mut cells[y * width + x];
            if cell[0] == 1.0 {
                bottom = y + sample_height;
            }
            cell[2] = if y < bottom { 1.0 } else { 0.0 };
        }
        check()?;
    }
    Ok(())
}

fn color_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    let [dl, da, db] = std::array::from_fn(|c| left[c] - right[c]);
    (dl * dl + da * da + db * db).sqrt()
}

// Symmetrically shrink the box at image boundaries, so truncation cannot create
// a false residual on a linear ramp. Both window endpoints only move forward.
fn average_line(
    dimensions: ImageDimensions,
    vertical: bool,
    line: usize,
    radius: usize,
    cells: &mut [[f32; 3]],
    queue: &mut [[f32; 3]],
) {
    let width = dimensions.width_usize();
    let length = if vertical {
        dimensions.height_usize()
    } else {
        width
    };
    let index = |position| {
        if vertical {
            position * width + line
        } else {
            line * width + position
        }
    };
    for position in 0..length {
        queue[position] = cells[index(position)];
    }
    let mut sum = [0.0_f64; 3];
    let mut left = 0;
    let mut right = 0;
    for position in 0..length {
        let span = radius.min(position).min(length - 1 - position);
        let start = position - span;
        let end = position + span + 1;
        while right < end {
            for c in 0..3 {
                sum[c] += f64::from(queue[right][c]);
            }
            right += 1;
        }
        while left < start {
            for c in 0..3 {
                sum[c] -= f64::from(queue[left][c]);
            }
            left += 1;
        }
        cells[index(position)] = sum.map(|v| (v / (end - start) as f64) as f32);
    }
}

// Each line index enters and leaves the monotone queue once. Validated image
// sides are at most 32768, so the f32 queue indices are exact integers.
fn maximum_line(
    dimensions: ImageDimensions,
    vertical: bool,
    line: usize,
    span: usize,
    cells: &mut [[f32; 3]],
    queue: &mut [[f32; 3]],
) {
    let width = dimensions.width_usize();
    let length = if vertical {
        dimensions.height_usize()
    } else {
        width
    };
    let mut head = 0;
    let mut tail = 0;
    for position in 0..length {
        let current = line_value(cells, width, vertical, line, position);
        while head < tail && queue[head][0] as usize + span <= position {
            head += 1;
        }
        while head < tail
            && line_value(cells, width, vertical, line, queue[tail - 1][0] as usize) <= current
        {
            tail -= 1;
        }
        queue[tail][0] = position as f32;
        tail += 1;
        if position + 1 < span {
            continue;
        }
        let maximum = line_value(cells, width, vertical, line, queue[head][0] as usize);
        let start = position + 1 - span;
        if vertical {
            cells[start * width + line][2] = maximum;
        } else {
            cells[line * width + start][0] = maximum;
        }
    }
}

fn line_value(
    cells: &[[f32; 3]],
    width: usize,
    vertical: bool,
    line: usize,
    position: usize,
) -> f32 {
    if vertical {
        cells[position * width + line][0]
    } else {
        cells[line * width + position][2]
    }
}
