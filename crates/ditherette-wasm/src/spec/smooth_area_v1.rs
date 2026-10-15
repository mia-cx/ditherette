//! Overlapping square-area contrast placement. See `smooth_area_v1.md`.

pub mod render;

use crate::image::{ImageDimensions, ImageView, Rgba8};
use crate::spec::color::oklab::rgb8_to_oklab;

/// Pixel records plus two colour rows, reused later as one line queue.
pub fn scratch_len(dimensions: ImageDimensions) -> usize {
    dimensions.pixel_count().expect("validated dimensions")
        + (2 * dimensions.width_usize()).max(dimensions.height_usize())
}

/// Selects complete overlapping squares whose largest local Oklab step is within tolerance.
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
    // The one-pixel read halo makes a strong edge fail every containing footprint,
    // including squares ending immediately beside it. It never grows with sample size.
    for y in 0..height {
        // Preserve the current and previous coordinate rows before overwriting dead
        // coordinates with sharpness. The next row still lives in the pixel records.
        let slot = y % 2;
        queue[slot * width..(slot + 1) * width].copy_from_slice(&cells[y * width..(y + 1) * width]);
        for x in 0..width {
            let center = queue[slot * width + x];
            let mut contrast = 0.0_f32;
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                let row = if ny <= y {
                    &queue[(ny % 2) * width..(ny % 2 + 1) * width]
                } else {
                    &cells[ny * width..(ny + 1) * width]
                };
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let [dl, da, db] = std::array::from_fn(|c| center[c] - row[nx][c]);
                    contrast = contrast.max((dl * dl + da * da + db * db).sqrt().min(1.0));
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
