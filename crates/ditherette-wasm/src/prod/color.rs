//! Production color-space materialization.
//!
//! These kernels convert browser RGBA8/sRGB pixels into packed four-channel
//! color buffers used by pipeline caches. The first three channels are the
//! requested color space; alpha is preserved as a normalized `0..=1` f32 value.

#[cfg(feature = "threads")]
use rayon::prelude::*;

use crate::{
    image::{ImageDimensions, ImageFormat, ImageView, Rgba8},
    prod::tiling::RowBand,
};

const COLOR_PARALLEL_PIXEL_THRESHOLD: usize = 40_000;
const DEFAULT_COLOR_ROW_BAND_HEIGHT: usize = 32;
const LARGE_PERCEPTUAL_COLOR_PIXEL_THRESHOLD: usize = 2_700_000;
const LARGE_PERCEPTUAL_COLOR_ROW_BAND_HEIGHT: usize = 64;

/// Supported f32 color-space materializations for RGBA8 input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpaceF32 {
    /// Gamma-encoded sRGB channels normalized to `0..=1`, plus alpha.
    Srgb,
    /// Linearized sRGB channels, plus alpha.
    LinearSrgb,
    /// Oklab L, a, b, plus alpha.
    Oklab,
    /// OKLCH lightness, chroma, hue-radians, plus alpha.
    Oklch,
    /// D65 CIELAB L*, a*, b*, plus alpha.
    Cielab,
    /// D65 CIELCH lightness, chroma, hue-radians, plus alpha.
    Cielch,
    /// Full-range BT.601 Y, Cb, Cr over gamma-encoded sRGB, plus alpha.
    YCbCr,
}

impl ColorSpaceF32 {
    /// Parses string enums used by the Wasm API.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "srgb" | "srgb32" | "srgb-f32" => Some(Self::Srgb),
            "linear-srgb" | "linear-srgb-f32" | "linear-rgba32" => Some(Self::LinearSrgb),
            "oklab" | "oklab-f32" | "oklaba32" => Some(Self::Oklab),
            "oklch" | "oklch-f32" | "oklch32" => Some(Self::Oklch),
            "cielab" | "cielab-f32" | "cielab32" => Some(Self::Cielab),
            "cielch" | "cielch-f32" | "cielch32" => Some(Self::Cielch),
            "ycbcr" | "ycbcr-f32" | "ycbcr32" => Some(Self::YCbCr),
            _ => None,
        }
    }
}

/// Materializes RGBA8/sRGB input into an AoS four-channel f32 color buffer.
///
/// Output channel order is defined by `target`; the fourth channel is always
/// normalized alpha. When threaded Wasm is enabled and
/// `parallelization_policy` is true, production policy selects the pooled
/// direct row-band adapter for images large enough to amortize worker overhead.
pub fn rgba8_to_color_space_f32(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    _parallelization_policy: bool,
) -> Vec<f32> {
    let dimensions = source.dimensions();
    let mut output = vec![0.0; color_output_len(dimensions)];
    rgba8_to_color_space_f32_with_policy_into(source, target, _parallelization_policy, &mut output);
    output
}

fn color_output_len(dimensions: ImageDimensions) -> usize {
    dimensions.pixel_count().expect("valid dimensions") * Rgba8::CHANNEL_COUNT
}

fn color_output_len_rgb(dimensions: ImageDimensions) -> usize {
    dimensions.pixel_count().expect("valid dimensions") * 3
}

/// Prototype-only scalar materialization into packed RGB triplets.
///
/// This deliberately bypasses the production four-channel image format so the
/// layout decision can be benchmarked without first changing that contract.
pub fn rgba8_to_color_space_f32x3_aos_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    output: &mut [f32],
) {
    assert_eq!(output.len(), color_output_len_rgb(source.dimensions()));
    let tables = ColorTables::new();

    for (source_pixel, output_pixel) in source
        .data()
        .chunks_exact(Rgba8::CHANNEL_COUNT)
        .zip(output.chunks_exact_mut(3))
    {
        let channels = convert_rgb(
            source_pixel[Rgba8::R],
            source_pixel[Rgba8::G],
            source_pixel[Rgba8::B],
            target,
            &tables,
        );
        output_pixel.copy_from_slice(&channels);
    }
}

/// Prototype-only scalar materialization into three contiguous channel planes.
pub fn rgba8_to_color_space_f32x3_soa_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    output: &mut [f32],
) {
    let pixel_count = source.dimensions().pixel_count().expect("valid dimensions");
    assert_eq!(output.len(), color_output_len_rgb(source.dimensions()));
    let (plane_0, remaining) = output.split_at_mut(pixel_count);
    let (plane_1, plane_2) = remaining.split_at_mut(pixel_count);
    let tables = ColorTables::new();

    for (index, source_pixel) in source.data().chunks_exact(Rgba8::CHANNEL_COUNT).enumerate() {
        let channels = convert_rgb(
            source_pixel[Rgba8::R],
            source_pixel[Rgba8::G],
            source_pixel[Rgba8::B],
            target,
            &tables,
        );
        plane_0[index] = channels[0];
        plane_1[index] = channels[1];
        plane_2[index] = channels[2];
    }
}

pub fn rgba8_to_color_space_f32_with_policy_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    parallelization_policy: bool,
    output: &mut [f32],
) {
    #[cfg(feature = "threads")]
    if let Some(policy) =
        ColorTilingPolicy::for_request(source.dimensions(), target, parallelization_policy)
    {
        rgba8_to_color_space_f32_parallel_with_band_height_into(
            source,
            target,
            output,
            policy.row_band_height,
        );
        return;
    }

    let _ = parallelization_policy;
    rgba8_to_color_space_f32_into(source, target, output);
}

pub fn rgba8_to_color_space_f32_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    output: &mut [f32],
) {
    let dimensions = source.dimensions();
    assert_eq!(output.len(), color_output_len(dimensions));

    let band = RowBand::new(0, dimensions.height()).expect("image height should be non-zero");
    let tables = ColorTables::new();
    rgba8_to_color_space_f32_rows_with_tables_into(source, target, band, output, &tables);
}

/// Materializes one absolute output row band into a full-image output buffer.
///
/// The band uses full-image row coordinates and writes only those rows. This is
/// the direct-write color conversion contract used by the Wasm-thread
/// prototype; callers own splitting row bands into non-overlapping assignments.
pub fn rgba8_to_color_space_f32_rows_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    row_band: RowBand,
    output: &mut [f32],
) {
    let dimensions = source.dimensions();
    assert_eq!(output.len(), color_output_len(dimensions));
    assert!(row_band.y_end() <= dimensions.height());

    let tables = ColorTables::new();
    rgba8_to_color_space_f32_rows_with_tables_into(source, target, row_band, output, &tables);
}

fn rgba8_to_color_space_f32_rows_with_tables_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    row_band: RowBand,
    output: &mut [f32],
    tables: &ColorTables,
) {
    let dimensions = source.dimensions();
    assert_eq!(output.len(), color_output_len(dimensions));
    assert!(row_band.y_end() <= dimensions.height());

    for y in row_band.y_start()..row_band.y_end() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_y_start = y as usize * dimensions.width_usize() * Rgba8::CHANNEL_COUNT;
        let output_row = &mut output
            [output_y_start..output_y_start + dimensions.width_usize() * Rgba8::CHANNEL_COUNT];
        materialize_color_row(source_row, target, output_row, tables);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTilingPolicy {
    row_band_height: usize,
}

impl ColorTilingPolicy {
    /// Selects the production color tiling policy from M4 browser/Wasm sweep data.
    ///
    /// The sweep showed pooled direct color conversion becomes reliably profitable
    /// around 40k output pixels, prefers all available Rayon workers up to the
    /// configured pool cap, and is best served by 32-row bands except for very
    /// large perceptual conversions where 64-row bands are slightly steadier.
    pub fn for_request(
        dimensions: ImageDimensions,
        target: ColorSpaceF32,
        parallelization_policy: bool,
    ) -> Option<Self> {
        if !parallelization_policy
            || dimensions.pixel_count().expect("valid dimensions") < COLOR_PARALLEL_PIXEL_THRESHOLD
        {
            return None;
        }

        let row_band_height = if is_perceptual_color_space(target)
            && dimensions.pixel_count().expect("valid dimensions")
                >= LARGE_PERCEPTUAL_COLOR_PIXEL_THRESHOLD
        {
            LARGE_PERCEPTUAL_COLOR_ROW_BAND_HEIGHT
        } else {
            DEFAULT_COLOR_ROW_BAND_HEIGHT
        };

        Some(Self { row_band_height })
    }

    pub const fn row_band_height(self) -> usize {
        self.row_band_height
    }
}

fn is_perceptual_color_space(target: ColorSpaceF32) -> bool {
    matches!(
        target,
        ColorSpaceF32::Oklab | ColorSpaceF32::Oklch | ColorSpaceF32::Cielab | ColorSpaceF32::Cielch
    )
}

#[cfg(feature = "threads")]
pub fn rgba8_to_color_space_f32_parallel_with_band_height_into(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    output: &mut [f32],
    row_band_height: usize,
) {
    let dimensions = source.dimensions();
    assert_eq!(output.len(), color_output_len(dimensions));
    let row_band_height = row_band_height.max(1);

    let row_len = dimensions.width_usize() * Rgba8::CHANNEL_COUNT;
    let band_len = row_len * row_band_height;
    let tables = ColorTables::new();

    output
        .par_chunks_mut(band_len)
        .enumerate()
        .for_each(|(band_index, output_band)| {
            let y_start = band_index * row_band_height;
            for (local_y, output_row) in output_band.chunks_mut(row_len).enumerate() {
                let source_row = source
                    .row((y_start + local_y) as u32)
                    .expect("parallel source row should be in bounds");
                materialize_color_row(source_row, target, output_row, &tables);
            }
        });
}

struct ColorTables {
    unit: [f32; 256],
    linear: [f32; 256],
}

impl ColorTables {
    fn new() -> Self {
        let mut unit = [0.0; 256];
        let mut linear = [0.0; 256];
        for channel in 0..=255 {
            let normalized = channel as f32 / 255.0;
            unit[channel] = normalized;
            linear[channel] = srgb_unit_to_linear(normalized);
        }
        Self { unit, linear }
    }

    fn unit(&self, channel: u8) -> f32 {
        self.unit[channel as usize]
    }

    fn linear(&self, channel: u8) -> f32 {
        self.linear[channel as usize]
    }
}

fn materialize_color_row(
    source_row: &[u8],
    target: ColorSpaceF32,
    output_row: &mut [f32],
    tables: &ColorTables,
) {
    for x in 0..source_row.len() / Rgba8::CHANNEL_COUNT {
        let source_start = x * Rgba8::CHANNEL_COUNT;
        let output_start = x * Rgba8::CHANNEL_COUNT;
        let r = source_row[source_start + Rgba8::R];
        let g = source_row[source_start + Rgba8::G];
        let b = source_row[source_start + Rgba8::B];
        let alpha = tables.unit(source_row[source_start + Rgba8::A]);
        let channels = convert_rgb(r, g, b, target, tables);

        output_row[output_start] = channels[0];
        output_row[output_start + 1] = channels[1];
        output_row[output_start + 2] = channels[2];
        output_row[output_start + 3] = alpha;
    }
}

fn convert_rgb(r: u8, g: u8, b: u8, target: ColorSpaceF32, tables: &ColorTables) -> [f32; 3] {
    match target {
        ColorSpaceF32::Srgb => [tables.unit(r), tables.unit(g), tables.unit(b)],
        ColorSpaceF32::LinearSrgb => [tables.linear(r), tables.linear(g), tables.linear(b)],
        ColorSpaceF32::Oklab => srgb8_to_oklab(r, g, b, tables),
        ColorSpaceF32::Oklch => {
            let [l, a, b] = srgb8_to_oklab(r, g, b, tables);
            cartesian_to_cylindrical(l, a, b)
        }
        ColorSpaceF32::Cielab => srgb8_to_cielab(r, g, b, tables),
        ColorSpaceF32::Cielch => {
            let [l, a, b] = srgb8_to_cielab(r, g, b, tables);
            cartesian_to_cylindrical(l, a, b)
        }
        ColorSpaceF32::YCbCr => srgb8_to_ycbcr(r, g, b, tables),
    }
}

fn srgb_unit_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_srgb_to_xyz(r: f32, g: f32, b: f32) -> [f32; 3] {
    [
        0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b,
        0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b,
        0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b,
    ]
}

fn srgb8_to_oklab(r: u8, g: u8, b: u8, tables: &ColorTables) -> [f32; 3] {
    linear_srgb_to_oklab(tables.linear(r), tables.linear(g), tables.linear(b))
}

fn linear_srgb_to_oklab(r: f32, g: f32, b: f32) -> [f32; 3] {
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    [
        0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_,
        1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_,
        0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    ]
}

fn srgb8_to_cielab(r: u8, g: u8, b: u8, tables: &ColorTables) -> [f32; 3] {
    let [x, y, z] = linear_srgb_to_xyz(tables.linear(r), tables.linear(g), tables.linear(b));
    xyz_to_cielab(x, y, z)
}

fn xyz_to_cielab(x: f32, y: f32, z: f32) -> [f32; 3] {
    const D65_XN: f32 = 0.95047;
    const D65_YN: f32 = 1.0;
    const D65_ZN: f32 = 1.08883;

    let fx = lab_f(x / D65_XN);
    let fy = lab_f(y / D65_YN);
    let fz = lab_f(z / D65_ZN);

    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_f(t: f32) -> f32 {
    const EPSILON: f32 = 216.0 / 24_389.0;
    const KAPPA: f32 = 24_389.0 / 27.0;

    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

fn cartesian_to_cylindrical(lightness: f32, a: f32, b: f32) -> [f32; 3] {
    let chroma = (a * a + b * b).sqrt();
    let hue = b.atan2(a).rem_euclid(std::f32::consts::TAU);
    [lightness, chroma, hue]
}

fn srgb8_to_ycbcr(r: u8, g: u8, b: u8, tables: &ColorTables) -> [f32; 3] {
    let r = tables.unit(r);
    let g = tables.unit(g);
    let b = tables.unit(b);
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    [y, 0.5 + (b - y) / 1.772, 0.5 + (r - y) / 1.402]
}
