//! Typed version-one requests and allocation-free image validation.
//!
//! A JS adapter checks raw property types before constructing these Rust values.
//! Recipe decoding rejects unknown fields and malformed tagged combinations.

use serde::{Deserialize, Serialize};

use crate::image::{contracts::PaletteEntry, ImageDimensions, ImageView, Rgba8};

use super::error::{DitheretteError, ErrorCode};

pub const RECIPE_VERSION: u32 = 1;
pub const MAX_SOURCE_SIDE: u32 = 32_768;
pub const MAX_OUTPUT_SIDE: u32 = 16_384;
pub const MAX_PIXELS: u64 = 67_108_864;
pub const MAX_PALETTE_ENTRIES: usize = 256;
pub const DEFAULT_MEMORY_LIMIT_BYTES: u64 = 1_610_612_736;
pub const MAX_MEMORY_LIMIT_BYTES: u64 = 2_147_483_648;

/// Reversible working coordinates. Weighted RGB choices are matching metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkingSpace {
    Srgb,
    LinearRgb,
    Oklab,
    Oklch,
    Cielab,
    Cielch,
    Ycbcr,
}

/// Exhaustive valid space/metric tags. Cylindrical Euclidean retains coordinate semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchPolicy {
    SrgbEuclidean,
    SrgbCompuphase,
    SrgbRec601,
    SrgbRec709,
    LinearRgbEuclidean,
    OklabEuclidean,
    OklchEuclidean,
    OklchCircularHue,
    OklchHueArc,
    CielabEuclidean,
    CielabCiede2000,
    CielchEuclidean,
    CielchCircularHue,
    CielchHueArc,
    YcbcrEuclidean,
}

impl MatchPolicy {
    /// Returns the coordinates consumed by this matching metric.
    pub const fn space(self) -> WorkingSpace {
        match self {
            Self::SrgbEuclidean | Self::SrgbCompuphase | Self::SrgbRec601 | Self::SrgbRec709 => {
                WorkingSpace::Srgb
            }
            Self::LinearRgbEuclidean => WorkingSpace::LinearRgb,
            Self::OklabEuclidean => WorkingSpace::Oklab,
            Self::OklchEuclidean | Self::OklchCircularHue | Self::OklchHueArc => {
                WorkingSpace::Oklch
            }
            Self::CielabEuclidean | Self::CielabCiede2000 => WorkingSpace::Cielab,
            Self::CielchEuclidean | Self::CielchCircularHue | Self::CielchHueArc => {
                WorkingSpace::Cielch
            }
            Self::YcbcrEuclidean => WorkingSpace::Ycbcr,
        }
    }
}

/// Decodes a plain-JS match tag, rejecting incoherent pairs before image allocation.
pub fn parse_match(tag: &str, path: &str) -> Result<MatchPolicy, DitheretteError> {
    MatchPolicy::deserialize(serde::de::value::StrDeserializer::<serde::de::value::Error>::new(tag))
        .map_err(|_| {
            DitheretteError::new(
                ErrorCode::InvalidSettings,
                path,
                "Unknown color/metric pair.",
            )
        })
}

/// All specified sampling anchors; area has no anchor because it integrates pixel footprints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

/// Convolution support for Catmull-Rom and Lanczos recipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Support {
    Fixed,
    ScaleAware,
}

/// Algorithm-specific settings prevent meaningless area anchors and nearest support policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "algorithm", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ResizePolicy {
    Nearest { anchor: Anchor },
    Area,
    Bilinear { anchor: Anchor },
    Bicubic { anchor: Anchor, support: Support },
    Lanczos2 { anchor: Anchor, support: Support },
    Lanczos3 { anchor: Anchor, support: Support },
    Trilinear { anchor: Anchor },
}

/// Requested output dimensions. The processor rejects excess dimensions rather than clamping them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub width: u32,
    pub height: u32,
    pub resize: ResizePolicy,
}

/// Alpha modes retain the website's byte threshold and concrete caller-selected matte RGB.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum AlphaPolicy {
    /// JavaScript-number precision matters at fractional byte thresholds.
    Preserve {
        threshold: f64,
    },
    Premultiplied,
    Matte {
        rgb: [u8; 3],
    },
}

/// Palette-independent contrast placement. Threshold/softness use the existing 0..100 contrast scale.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Placement {
    Everywhere,
    Adaptive {
        radius: u32,
        threshold: f32,
        softness: f32,
    },
}

/// Supported Bayer matrix widths for fields and two-color ordered mixing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BayerSize {
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "4")]
    Four,
    #[serde(rename = "8")]
    Eight,
    #[serde(rename = "16")]
    Sixteen,
}

/// Only these algorithms can produce RGBA8 without palette feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "algorithm", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Field {
    Bayer { size: BayerSize },
    Random { seed: u32 },
    BlueNoise,
}

/// Scalar scan-order-sensitive diffusion kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Diffusion {
    FloydSteinberg,
    Sierra,
    SierraLite,
    Atkinson,
}

/// Distinguishes byte-rounded sRGB feedback from unrounded matching coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffusionFeedback {
    SrgbBytes,
    Matching,
}

/// Palette-free perturbation uses normalized strength (website percentage divided by 100).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerturbPolicy {
    pub field: Field,
    pub space: WorkingSpace,
    pub strength: f32,
    pub placement: Placement,
}

/// Every dither family. A separable field explicitly chooses its working space independently of matching.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DitherPolicy {
    None,
    Separable {
        perturb: PerturbPolicy,
    },
    Diffusion {
        kernel: Diffusion,
        strength: f32,
        placement: Placement,
        serpentine: bool,
        feedback: DiffusionFeedback,
    },
    Yliluoma {
        size: BayerSize,
        placement: Placement,
    },
}

/// Complete normalized setting vocabulary for version one. No frontend identity or execution policy is a recipe field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeV1 {
    pub version: u32,
    pub output: Output,
    pub alpha: AlphaPolicy,
    #[serde(rename = "match")]
    pub matching: MatchPolicy,
    pub dither: DitherPolicy,
}

/// Decodes recipe structure; numeric range checks run when the request is validated.
pub fn decode_recipe(json: &str) -> Result<RecipeV1, DitheretteError> {
    serde_json::from_str(json).map_err(|_| {
        DitheretteError::new(
            ErrorCode::InvalidSettings,
            "recipe",
            "Malformed recipe or unsupported setting tag.",
        )
    })
}

/// Borrowed cropped RGBA8 input. Validation never mutates or retains the caller's buffer.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub width: u32,
    pub height: u32,
    pub data: &'a [u8],
}

/// End-to-end processing request; progress is an execution callback, not recipe identity.
#[derive(Debug, Clone, Copy)]
pub struct ProcessRequest<'a> {
    pub source: Source<'a>,
    pub palette: &'a [PaletteEntry],
    pub recipe: RecipeV1,
}

/// Resize-only request producing owned RGBA8.
#[derive(Debug, Clone, Copy)]
pub struct ResizeRequest<'a> {
    pub version: u32,
    pub source: Source<'a>,
    pub output: Output,
}

/// Palette-independent RGBA8 perturbation request; source alpha passes through unchanged.
#[derive(Debug, Clone, Copy)]
pub struct PerturbRequest<'a> {
    pub version: u32,
    pub source: Source<'a>,
    pub perturb: PerturbPolicy,
}

/// Direct matching request producing indexed output.
#[derive(Debug, Clone, Copy)]
pub struct QuantizeRequest<'a> {
    pub version: u32,
    pub source: Source<'a>,
    pub palette: &'a [PaletteEntry],
    pub alpha: AlphaPolicy,
    pub matching: MatchPolicy,
}

/// Fused dither/matching request producing indexed output.
#[derive(Debug, Clone, Copy)]
pub struct DitherQuantizeRequest<'a> {
    pub quantize: QuantizeRequest<'a>,
    pub dither: DitherPolicy,
}

/// Public operation inventory with no production dispatch.
#[derive(Debug, Clone, Copy)]
pub enum Request<'a> {
    Process(ProcessRequest<'a>),
    Resize(ResizeRequest<'a>),
    Perturb(PerturbRequest<'a>),
    Quantize(QuantizeRequest<'a>),
    DitherAndQuantize(DitherQuantizeRequest<'a>),
}

/// Validated storage views and output dimensions, created without output allocation.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedLayout<'a> {
    pub source: ImageView<'a, Rgba8>,
    pub output: ImageDimensions,
}

impl<'a> Request<'a> {
    /// Validates version/settings first, then source/output layout. No partial result is allocated.
    pub fn validate(self) -> Result<ValidatedLayout<'a>, DitheretteError> {
        let (source, output) = match self {
            Self::Process(request) => {
                validate_version(request.recipe.version, "recipe.version")?;
                validate_palette(request.palette)?;
                validate_alpha(request.recipe.alpha, "recipe.alpha")?;
                validate_dither(request.recipe.dither, "recipe.dither")?;
                (request.source, Some(request.recipe.output))
            }
            Self::Resize(request) => {
                validate_version(request.version, "version")?;
                (request.source, Some(request.output))
            }
            Self::Perturb(request) => {
                validate_version(request.version, "version")?;
                validate_perturb(request.perturb, "perturb")?;
                (request.source, None)
            }
            Self::Quantize(request) => {
                validate_quantize(request)?;
                (request.source, None)
            }
            Self::DitherAndQuantize(request) => {
                validate_quantize(request.quantize)?;
                validate_dither(request.dither, "dither")?;
                (request.quantize.source, None)
            }
        };
        let dimensions = validate_dimensions(
            source.width,
            source.height,
            MAX_SOURCE_SIDE,
            "source",
            ErrorCode::InvalidImage,
        )?;
        let view = ImageView::packed(source.data, dimensions).map_err(|_| {
            DitheretteError::new(
                ErrorCode::InvalidImage,
                "source.data",
                "RGBA8 byte length does not match dimensions.",
            )
        })?;
        let (width, height, path) = output.map_or((source.width, source.height, "source"), |out| {
            (out.width, out.height, "output")
        });
        let output = validate_dimensions(
            width,
            height,
            MAX_OUTPUT_SIDE,
            path,
            ErrorCode::InvalidSettings,
        )?;
        Ok(ValidatedLayout {
            source: view,
            output,
        })
    }
}

/// Applies existing image limits with overflow-safe arithmetic and stable field paths.
pub fn validate_dimensions(
    width: u32,
    height: u32,
    side_limit: u32,
    path: &str,
    code: ErrorCode,
) -> Result<ImageDimensions, DitheretteError> {
    for (name, value) in [("width", width), ("height", height)] {
        if value == 0 || value > side_limit {
            return Err(DitheretteError::new(
                code,
                format!("{path}.{name}"),
                "Image side is outside the supported range.",
            ));
        }
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(DitheretteError::new(
            code,
            path,
            "Image exceeds the maximum pixel count.",
        ));
    }
    ImageDimensions::new(width, height)
        .map_err(|error| DitheretteError::new(code, path, error.to_string()))
}

fn validate_version(version: u32, path: &str) -> Result<(), DitheretteError> {
    if version != RECIPE_VERSION {
        return Err(DitheretteError::new(
            ErrorCode::InvalidRequest,
            path,
            "Unsupported recipe version.",
        ));
    }
    Ok(())
}

fn validate_palette(palette: &[PaletteEntry]) -> Result<(), DitheretteError> {
    if palette.is_empty() {
        return Err(DitheretteError::new(
            ErrorCode::InvalidPalette,
            "palette",
            "Supply at least one visible color or Transparent.",
        ));
    }
    // S09 truncates to 256 and emits metadata. Oversize is valid under decision 40.
    Ok(())
}

fn validate_quantize(request: QuantizeRequest<'_>) -> Result<(), DitheretteError> {
    validate_version(request.version, "version")?;
    validate_palette(request.palette)?;
    validate_alpha(request.alpha, "alpha")
}

fn validate_alpha(alpha: AlphaPolicy, path: &str) -> Result<(), DitheretteError> {
    if let AlphaPolicy::Preserve { threshold } = alpha {
        if !threshold.is_finite() || !(0.0..=255.0).contains(&threshold) {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.threshold"),
                "Alpha threshold must be finite and between 0 and 255.",
            ));
        }
    }
    Ok(())
}

fn nonnegative(value: f32, path: String) -> Result<(), DitheretteError> {
    if !value.is_finite() || value < 0.0 {
        return Err(DitheretteError::new(
            ErrorCode::InvalidSettings,
            path,
            "Value must be finite and nonnegative.",
        ));
    }
    Ok(())
}

fn validate_placement(placement: Placement, path: &str) -> Result<(), DitheretteError> {
    if let Placement::Adaptive {
        radius,
        threshold,
        softness,
    } = placement
    {
        if radius == 0 || radius > MAX_SOURCE_SIDE {
            return Err(DitheretteError::new(
                ErrorCode::InvalidSettings,
                format!("{path}.radius"),
                "Adaptive radius must be between 1 and 32768 pixels.",
            ));
        }
        nonnegative(threshold, format!("{path}.threshold"))?;
        nonnegative(softness, format!("{path}.softness"))?;
    }
    Ok(())
}

fn validate_perturb(perturb: PerturbPolicy, path: &str) -> Result<(), DitheretteError> {
    nonnegative(perturb.strength, format!("{path}.strength"))?;
    validate_placement(perturb.placement, &format!("{path}.placement"))
}

fn validate_dither(dither: DitherPolicy, path: &str) -> Result<(), DitheretteError> {
    match dither {
        DitherPolicy::None => Ok(()),
        DitherPolicy::Separable { perturb } => {
            validate_perturb(perturb, &format!("{path}.perturb"))
        }
        DitherPolicy::Diffusion {
            strength,
            placement,
            ..
        } => {
            nonnegative(strength, format!("{path}.strength"))?;
            validate_placement(placement, &format!("{path}.placement"))
        }
        DitherPolicy::Yliluoma { placement, .. } => {
            validate_placement(placement, &format!("{path}.placement"))
        }
    }
}
