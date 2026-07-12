//! Error-diffusion dithering specs.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::common::{
    add_error, assert_dither_inputs, nearest_euclidean, read_color, sub_color, write_index,
    Palette3,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorDiffusionKernel {
    FloydSteinberg,
    Sierra,
    SierraLite,
    Atkinson,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffusionTap {
    pub dx: i32,
    pub dy: u32,
    pub weight: f32,
}

pub const FLOYD_STEINBERG_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 7.0 / 16.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 3.0 / 16.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 5.0 / 16.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 1,
        weight: 1.0 / 16.0,
    },
];

pub const SIERRA_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 5.0 / 32.0,
    },
    DiffusionTap {
        dx: 2,
        dy: 0,
        weight: 3.0 / 32.0,
    },
    DiffusionTap {
        dx: -2,
        dy: 1,
        weight: 2.0 / 32.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 4.0 / 32.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 5.0 / 32.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 1,
        weight: 4.0 / 32.0,
    },
    DiffusionTap {
        dx: 2,
        dy: 1,
        weight: 2.0 / 32.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 2,
        weight: 2.0 / 32.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 2,
        weight: 3.0 / 32.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 2,
        weight: 2.0 / 32.0,
    },
];

pub const SIERRA_LITE_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 2.0 / 4.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 1.0 / 4.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 1.0 / 4.0,
    },
];

pub const ATKINSON_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 2,
        dy: 0,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 1,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 2,
        weight: 1.0 / 8.0,
    },
];

impl ErrorDiffusionKernel {
    pub const fn taps(self) -> &'static [DiffusionTap] {
        match self {
            Self::FloydSteinberg => FLOYD_STEINBERG_TAPS,
            Self::Sierra => SIERRA_TAPS,
            Self::SierraLite => SIERRA_LITE_TAPS,
            Self::Atkinson => ATKINSON_TAPS,
        }
    }
}

pub fn dither_error_diffusion_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    kernel: ErrorDiffusionKernel,
    strength: f32,
    serpentine: bool,
) {
    dither_error_diffusion_by_nearest_into(
        source,
        palette,
        output,
        kernel,
        strength,
        serpentine,
        nearest_euclidean,
    );
}

pub fn dither_error_diffusion_by_nearest_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    kernel: ErrorDiffusionKernel,
    strength: f32,
    serpentine: bool,
    nearest: impl Fn([f32; 3], Palette3<'_>) -> (u8, [f32; 3]) + Copy,
) {
    assert_dither_inputs(&source, palette, &output);
    let dimensions = source.dimensions();
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    let mut errors = vec![[0.0; 3]; width * height];

    for y in 0..height {
        let reverse = serpentine && y % 2 == 1;
        let source_row = source
            .row(y as u32)
            .expect("source row should be in bounds");
        let output_row = output
            .row_mut(y as u32)
            .expect("output row should be in bounds");
        let xs: Box<dyn Iterator<Item = usize>> = if reverse {
            Box::new((0..width).rev())
        } else {
            Box::new(0..width)
        };

        for x in xs {
            let offset = y * width + x;
            let color = add_error(read_color::<F>(source_row, x), errors[offset]);
            let (index, quantized) = nearest(color, palette);
            write_index(output_row, x, index);
            let error = sub_color(color, quantized);

            for tap in kernel.taps() {
                let dx = if reverse { -tap.dx } else { tap.dx };
                let Some(target_x) = x.checked_add_signed(dx as isize) else {
                    continue;
                };
                let target_y = y + tap.dy as usize;
                if target_x >= width || target_y >= height {
                    continue;
                }
                let target = &mut errors[target_y * width + target_x];
                target[0] += error[0] * tap.weight * strength;
                target[1] += error[1] * tap.weight * strength;
                target[2] += error[2] * tap.weight * strength;
            }
        }
    }
}
