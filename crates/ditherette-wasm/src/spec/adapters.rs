//! Reference compositions for inherited executable adapters.
//!
//! These are not new public recipes. Legacy color uses f32x4 with normalized alpha,
//! including its uncorrected cylindrical neutral convention. Legacy process only resizes.
//! Diagnostic copy/noop adapters describe storage writes, not color or resize semantics.
//! No clocks, worker threads, lookup tables, prepared plans, or production imports occur here.

use serde::Deserialize;

use super::{
    color,
    contract::request::WorkingSpace,
    resize::{
        common::alignment::ResizeAnchor,
        scalar::{self, convolution::SupportPolicy},
    },
    tiling::{RowBand, RowBandPlan},
};
use crate::image::{ImageDimensions, ImageView, ImageViewMut, Rgba8};

/// Observable legacy greeting, outside the five public processing methods.
pub fn legacy_hello(name: &str) -> String {
    format!("Hello, {name}, from Ditherette's fresh Rust core!")
}

/// Parses the inherited Wasm target aliases without adding public recipe aliases.
pub fn legacy_color_space(value: &str) -> Option<WorkingSpace> {
    match value {
        "srgb" | "srgb32" | "srgb-f32" => Some(WorkingSpace::Srgb),
        "linear-srgb" | "linear-srgb-f32" | "linear-rgba32" => Some(WorkingSpace::LinearRgb),
        "oklab" | "oklab-f32" | "oklaba32" => Some(WorkingSpace::Oklab),
        "oklch" | "oklch-f32" | "oklch32" => Some(WorkingSpace::Oklch),
        "cielab" | "cielab-f32" | "cielab32" => Some(WorkingSpace::Cielab),
        "cielch" | "cielch-f32" | "cielch32" => Some(WorkingSpace::Cielch),
        "ycbcr" | "ycbcr-f32" | "ycbcr32" => Some(WorkingSpace::Ycbcr),
        _ => None,
    }
}

/// Validates the legacy byte input and allocates its owned, four-channel f32 result.
pub fn legacy_convert_color_space(
    input: &[u8],
    width: u32,
    height: u32,
    from: &str,
    to: &str,
) -> Result<Vec<f32>, String> {
    if !matches!(from, "rgba8" | "srgb-rgba8" | "srgb") {
        return Err("unsupported source color space".into());
    }
    let source = legacy_source(input, width, height)?;
    let space = legacy_color_space(to).ok_or("unsupported target color space")?;
    let mut output = vec![0.0; input.len()];
    legacy_color_rows_into(source, space, RowBand::new(0, height).unwrap(), &mut output);
    Ok(output)
}

/// Converts selected global rows into a full-image f32x4 buffer, leaving other rows unchanged.
/// Scalar, pooled-direct, policy-selected and explicit-band color adapters share this oracle.
pub fn legacy_color_rows_into(
    source: ImageView<'_, Rgba8>,
    space: WorkingSpace,
    band: RowBand,
    output: &mut [f32],
) {
    let dimensions = source.dimensions();
    let row_len = dimensions.width_usize() * 4;
    assert_eq!(output.len(), dimensions.height_usize() * row_len);
    assert!(band.y_end() <= dimensions.height());
    for y in band.y_start()..band.y_end() {
        for (x, pixel) in source.row(y).unwrap().chunks_exact(4).enumerate() {
            let rgb = [pixel[0], pixel[1], pixel[2]];
            let coordinates = match space {
                WorkingSpace::Oklch | WorkingSpace::Cielch => {
                    let [l, a, b] = if space == WorkingSpace::Oklch {
                        color::oklab::rgb8_to_oklab(rgb)
                    } else {
                        color::cielab::rgb8_to_cielab(rgb)
                    };
                    // Legacy conversion neither neutralizes byte grays nor maps rounded TAU to zero.
                    [
                        l,
                        (a * a + b * b).sqrt(),
                        b.atan2(a).rem_euclid(std::f32::consts::TAU),
                    ]
                }
                _ => color::rgb8_to_coordinates(rgb, space),
            };
            let offset = y as usize * row_len + x * 4;
            output[offset..offset + 3].copy_from_slice(&coordinates);
            output[offset + 3] = f32::from(pixel[3]) / 255.0;
        }
    }
}

/// Execution-only switches carried by the inherited support-policy string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyExecution {
    PolicyDefault,
    PooledDirect,
    PooledCopy,
    PooledNoop,
}

/// Parsed legacy resize request. Preparation scope changes work, not reference pixels.
#[derive(Debug, Clone, Copy)]
pub struct LegacyResize {
    filter: LegacyFilter,
    anchor: ResizeAnchor,
    support: SupportPolicy,
    pub execution: LegacyExecution,
    pub per_band_plan: bool,
}

#[derive(Debug, Clone, Copy)]
enum LegacyFilter {
    Nearest,
    Area,
    Bilinear,
    Bicubic,
    Lanczos2,
    Lanczos3,
}

impl LegacyResize {
    /// Preserves legacy filter, anchor and support aliases and validation order.
    pub fn parse(filter: &str, anchor: &str, support: &str) -> Result<Self, String> {
        let filter = match filter {
            "nearest" => LegacyFilter::Nearest,
            "area" => LegacyFilter::Area,
            "bilinear" => LegacyFilter::Bilinear,
            "bicubic" | "bicubic-catmull-rom" => LegacyFilter::Bicubic,
            "lanczos2" => LegacyFilter::Lanczos2,
            "lanczos3" => LegacyFilter::Lanczos3,
            _ => return Err("unsupported resize filter".into()),
        };
        let (support, execution, per_band_plan) = match support {
            "" | "fixed" => (SupportPolicy::Fixed, LegacyExecution::PolicyDefault, false),
            "scale-aware" | "scale_aware" => (
                SupportPolicy::ScaleAware,
                LegacyExecution::PolicyDefault,
                false,
            ),
            "fixed+pooled-direct" => (SupportPolicy::Fixed, LegacyExecution::PooledDirect, false),
            "fixed+pooled-direct+per-band-plan" => {
                (SupportPolicy::Fixed, LegacyExecution::PooledDirect, true)
            }
            "fixed+pooled-copy" => (SupportPolicy::Fixed, LegacyExecution::PooledCopy, false),
            "fixed+pooled-noop" => (SupportPolicy::Fixed, LegacyExecution::PooledNoop, false),
            _ => return Err("unsupported support policy".into()),
        };
        let anchor = match anchor {
            "" | "center" => ResizeAnchor::Center,
            "top-left" => ResizeAnchor::TopLeft,
            "top" => ResizeAnchor::Top,
            "top-right" => ResizeAnchor::TopRight,
            "left" => ResizeAnchor::Left,
            "right" => ResizeAnchor::Right,
            "bottom-left" => ResizeAnchor::BottomLeft,
            "bottom" => ResizeAnchor::Bottom,
            "bottom-right" => ResizeAnchor::BottomRight,
            _ => return Err("unsupported resize anchor".into()),
        };
        Ok(Self {
            filter,
            anchor,
            support,
            execution,
            per_band_plan,
        })
    }

    /// Direct reference pixels, independent of preparation or execution settings.
    pub fn resize_into(self, source: ImageView<'_, Rgba8>, output: ImageViewMut<'_, Rgba8>) {
        match self.filter {
            LegacyFilter::Nearest => {
                scalar::nearest::resize_nearest_into(source, output, self.anchor)
            }
            LegacyFilter::Area => scalar::area::resize_area_into(source, output),
            LegacyFilter::Bilinear => {
                scalar::bilinear::resize_bilinear_into(source, output, self.anchor)
            }
            LegacyFilter::Bicubic => {
                scalar::bicubic::resize_bicubic_into(source, output, self.anchor, self.support)
            }
            LegacyFilter::Lanczos2 => {
                scalar::lanczos::resize_lanczos2_into(source, output, self.anchor, self.support)
            }
            LegacyFilter::Lanczos3 => {
                scalar::lanczos::resize_lanczos3_into(source, output, self.anchor, self.support)
            }
        }
    }

    /// Computes the whole naive image, then copies selected global rows into a local band buffer.
    /// Planned, per-band-plan and direct-write resize adapters all project this same image.
    pub fn resize_rows_into(
        self,
        source: ImageView<'_, Rgba8>,
        dimensions: ImageDimensions,
        band: RowBand,
        output: &mut [u8],
    ) {
        assert!(band.y_end() <= dimensions.height());
        let row_len = dimensions.width_usize() * 4;
        assert_eq!(output.len(), band.height() as usize * row_len);
        let mut whole = vec![0; dimensions.storage_len::<Rgba8>().unwrap()];
        self.resize_into(
            source,
            ImageViewMut::packed(&mut whole, dimensions).unwrap(),
        );
        output.copy_from_slice(
            &whole[band.y_start() as usize * row_len..band.y_end() as usize * row_len],
        );
    }

    /// Sequential reference for the legacy pooled-direct filter set; worker count is irrelevant.
    pub fn pooled_direct_into(
        self,
        source: ImageView<'_, Rgba8>,
        dimensions: ImageDimensions,
        band_height: u32,
        output: &mut [u8],
    ) -> Result<(), String> {
        if matches!(self.filter, LegacyFilter::Area | LegacyFilter::Bilinear) {
            return Err("resize filter does not support pooled direct".into());
        }
        assert_eq!(output.len(), dimensions.storage_len::<Rgba8>().unwrap());
        let plan = RowBandPlan::for_output_height(dimensions, band_height.max(1)).unwrap();
        let row_len = dimensions.width_usize() * 4;
        for &band in plan.bands() {
            self.resize_rows_into(
                source,
                dimensions,
                band,
                &mut output[band.y_start() as usize * row_len..band.y_end() as usize * row_len],
            );
        }
        Ok(())
    }
}

/// Legacy allocation/validation composition. Diagnostic modes apply only when policy is enabled.
/// The legacy scalar boundary uses 32-row prototype bands and has no version-one size ceilings.
#[allow(clippy::too_many_arguments)]
pub fn legacy_resize_rgba8(
    input: &[u8],
    width: u32,
    height: u32,
    output_width: u32,
    output_height: u32,
    filter: &str,
    anchor: &str,
    support: &str,
    parallelization_policy: bool,
) -> Result<Vec<u8>, String> {
    let source_dimensions = ImageDimensions::new(width, height).map_err(|e| e.to_string())?;
    let dimensions =
        ImageDimensions::new(output_width, output_height).map_err(|e| e.to_string())?;
    let source = legacy_source(input, source_dimensions.width(), source_dimensions.height())?;
    let resize = LegacyResize::parse(filter, anchor, support)?;
    let mut output = vec![
        0;
        dimensions
            .storage_len::<Rgba8>()
            .map_err(|e| e.to_string())?
    ];
    let execution = if parallelization_policy {
        resize.execution
    } else {
        LegacyExecution::PolicyDefault
    };
    match execution {
        LegacyExecution::PolicyDefault => resize.resize_into(
            source,
            ImageViewMut::packed(&mut output, dimensions).unwrap(),
        ),
        LegacyExecution::PooledDirect => {
            resize.pooled_direct_into(source, dimensions, 32, &mut output)?
        }
        LegacyExecution::PooledCopy => {
            diagnostic_resize_copy_into(source, dimensions, 32, &mut output)
        }
        LegacyExecution::PooledNoop => diagnostic_noop(&mut output),
    }
    Ok(output)
}

#[derive(Deserialize)]
struct LegacySettings {
    output: LegacyOutput,
}
#[derive(Deserialize)]
struct LegacyOutput {
    width: u32,
    height: u32,
    resize: String,
    crop: Option<LegacyCrop>,
}
#[derive(Deserialize)]
struct LegacyCrop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

/// The inherited process export parses app JSON, accepts only a full-source crop, and resizes.
/// Unknown JSON fields remain accepted. This returns RGBA8, never an indexed public process result.
pub fn legacy_process_rgba8(
    input: &[u8],
    width: u32,
    height: u32,
    settings_json: &str,
    parallelization_policy: bool,
) -> Result<Vec<u8>, String> {
    let settings: LegacySettings = serde_json::from_str(settings_json)
        .map_err(|e| format!("invalid process settings: {e}"))?;
    let output = settings.output;
    if let Some(crop) = output.crop {
        if crop.x != 0 || crop.y != 0 || crop.width != width || crop.height != height {
            return Err("processRgba8 does not support cropped sources yet".into());
        }
    }
    let (filter, support) = match output.resize.as_str() {
        "nearest"
        | "area"
        | "bilinear"
        | "bicubic"
        | "bicubic-catmull-rom"
        | "lanczos2"
        | "lanczos3" => (output.resize.as_str(), "fixed"),
        "lanczos2-scale-aware" => ("lanczos2", "scale-aware"),
        "lanczos3-scale-aware" => ("lanczos3", "scale-aware"),
        _ => return Err("unsupported process resize mode".into()),
    };
    legacy_resize_rgba8(
        input,
        width,
        height,
        output.width,
        output.height,
        filter,
        "center",
        support,
        parallelization_policy,
    )
}

/// Preserves every initialized output element. Both inherited pooled-noop diagnostics use this identity.
pub fn diagnostic_noop<T>(_output: &mut [T]) {}

/// Casts logical RGBA bytes directly to f32, without color conversion or alpha normalization.
pub fn diagnostic_color_copy_into(source: ImageView<'_, Rgba8>, output: &mut [f32]) {
    let row_len = source.dimensions().width_usize() * 4;
    assert_eq!(output.len(), source.dimensions().height_usize() * row_len);
    for y in 0..source.dimensions().height() {
        for (target, byte) in output[y as usize * row_len..(y as usize + 1) * row_len]
            .iter_mut()
            .zip(source.row(y).unwrap())
        {
            *target = f32::from(*byte);
        }
    }
}

/// Repeats raw source storage using the legacy band's own byte length in its source offset.
/// Padding is source data here; the shortened last band can start at a different repeat phase.
/// This diagnostic deliberately depends on band height and is not a resize recipe.
pub fn diagnostic_resize_copy_into(
    source: ImageView<'_, Rgba8>,
    dimensions: ImageDimensions,
    band_height: u32,
    output: &mut [u8],
) {
    assert_eq!(output.len(), dimensions.storage_len::<Rgba8>().unwrap());
    let row_len = dimensions.width_usize() * 4;
    let plan = RowBandPlan::for_output_height(dimensions, band_height.max(1)).unwrap();
    for band in plan.bands() {
        let destination =
            &mut output[band.y_start() as usize * row_len..band.y_end() as usize * row_len];
        // Retains the inherited usize expression, including its build/target overflow behavior.
        let offset = (band.y_start() as usize * destination.len()) % source.data().len();
        for (index, byte) in destination.iter_mut().enumerate() {
            *byte = source.data()[(offset + index) % source.data().len()];
        }
    }
}

fn legacy_source(input: &[u8], width: u32, height: u32) -> Result<ImageView<'_, Rgba8>, String> {
    let dimensions = ImageDimensions::new(width, height).map_err(|e| e.to_string())?;
    if input.len()
        != dimensions
            .storage_len::<Rgba8>()
            .map_err(|e| e.to_string())?
    {
        return Err("input length does not match RGBA8 dimensions".into());
    }
    ImageView::packed(input, dimensions).map_err(|e| e.to_string())
}
