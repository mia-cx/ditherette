//! Adapters from concrete request/storage contracts to the shared conformance model.

use crate::{
    image::{
        contracts::{IndexedImage, Rgba8Image, WarningCode as PublicWarningCode},
        ImageBuf, ImageFormat,
    },
    spec::contract::request::{
        Anchor, Request, ResizePolicy, ResizeRequest, Support, WorkingSpace,
    },
};
use ditherette_bench_api::{
    verification::{ColorSpace, Dimensions, Pixels, VerificationOutput, Warning, WarningCode},
    BenchSubject, BenchSubjectError, ResizeAnchorParam, ResizeInputU8Rgba, ResizeOutputU8Rgba,
    ResizeParams, SupportPolicyParam,
};

/// Preserve public palette order, transparency, warning codes/messages, and logical indices.
pub fn indexed_output(image: &IndexedImage) -> VerificationOutput {
    VerificationOutput {
        dimensions: dimensions(&image.indices),
        pixels: Pixels::Indexed8 {
            indices: packed(&image.indices),
            palette_rgba: image.palette.rgba.clone(),
            transparent_index: image.palette.transparent_index,
        },
        warnings: image
            .warnings
            .iter()
            .map(|warning| Warning {
                code: match warning.code {
                    PublicWarningCode::PaletteTruncated => WarningCode::PaletteTruncated,
                    PublicWarningCode::TransparentOnly => WarningCode::TransparentOnly,
                    PublicWarningCode::TransparentFallback => WarningCode::TransparentFallback,
                },
                message: warning.message.clone(),
            })
            .collect(),
    }
}

/// Copy logical RGBA pixels without including row padding in conformance identity.
pub fn rgba_output(image: &Rgba8Image) -> VerificationOutput {
    VerificationOutput {
        dimensions: dimensions(image),
        pixels: Pixels::Rgba8 {
            data: packed(image),
        },
        warnings: Vec::new(),
    }
}

/// Map the public working-space tag without inferring it from coordinate values.
pub fn color_space(space: WorkingSpace) -> ColorSpace {
    match space {
        WorkingSpace::Srgb => ColorSpace::Srgb,
        WorkingSpace::LinearRgb => ColorSpace::LinearRgb,
        WorkingSpace::Oklab => ColorSpace::Oklab,
        WorkingSpace::Oklch => ColorSpace::Oklch,
        WorkingSpace::Cielab => ColorSpace::Cielab,
        WorkingSpace::Cielch => ColorSpace::Cielch,
        WorkingSpace::Ycbcr => ColorSpace::Ycbcr,
    }
}

/// Execute the complete readable resize request, including its validation.
pub fn reference_resize(
    request: &ResizeRequest<'_>,
) -> Result<VerificationOutput, BenchSubjectError> {
    crate::spec::resize::resize(*request)
        .map(|image| rgba_output(&image))
        .map_err(|error| BenchSubjectError::new(error.to_string()))
}

/// Execute the existing production resize subject, or report a missing implementation.
pub fn production_resize(
    request: &ResizeRequest<'_>,
) -> Result<VerificationOutput, BenchSubjectError> {
    resize("prod", request)
}

fn resize(
    module: &str,
    request: &ResizeRequest<'_>,
) -> Result<VerificationOutput, BenchSubjectError> {
    Request::Resize(*request)
        .validate()
        .map_err(|error| BenchSubjectError::new(error.to_string()))?;
    let (filter, variant, anchor, support) = match request.output.resize {
        ResizePolicy::Nearest { anchor } => ("nearest", "scalar", Some(anchor), None),
        ResizePolicy::Area => ("area", "scalar", None, None),
        ResizePolicy::Bilinear { anchor } => ("bilinear", "scalar", Some(anchor), None),
        ResizePolicy::Bicubic { anchor, support } => (
            "bicubic",
            match support {
                Support::Fixed => "catmull-rom",
                Support::ScaleAware => "catmull-rom-scale-aware",
            },
            Some(anchor),
            Some(support),
        ),
        ResizePolicy::Lanczos2 { anchor, support } => (
            "lanczos2",
            support_name(support),
            Some(anchor),
            Some(support),
        ),
        ResizePolicy::Lanczos3 { anchor, support } => (
            "lanczos3",
            support_name(support),
            Some(anchor),
            Some(support),
        ),
        ResizePolicy::Trilinear { anchor } => ("trilinear", "mip-area", Some(anchor), None),
    };
    let id = format!("{module}:resize:{filter}:{variant}");
    let subject = super::bench_subjects()
        .into_iter()
        .find_map(|subject| match subject {
            BenchSubject::Resize(subject) if subject.descriptor.id.as_str() == id => Some(subject),
            _ => None,
        })
        .ok_or_else(|| {
            BenchSubjectError::new(format!("missing required resize implementation {id}"))
        })?;
    let params = ResizeParams {
        anchor: anchor.map_or(ResizeAnchorParam::Center, |anchor| match anchor {
            Anchor::TopLeft => ResizeAnchorParam::TopLeft,
            Anchor::Top => ResizeAnchorParam::Top,
            Anchor::TopRight => ResizeAnchorParam::TopRight,
            Anchor::Left => ResizeAnchorParam::Left,
            Anchor::Center => ResizeAnchorParam::Center,
            Anchor::Right => ResizeAnchorParam::Right,
            Anchor::BottomLeft => ResizeAnchorParam::BottomLeft,
            Anchor::Bottom => ResizeAnchorParam::Bottom,
            Anchor::BottomRight => ResizeAnchorParam::BottomRight,
        }),
        support_policy: support.map(|support| match support {
            Support::Fixed => SupportPolicyParam::Fixed,
            Support::ScaleAware => SupportPolicyParam::ScaleAware,
        }),
        ..ResizeParams::default()
    };
    let dimensions = Dimensions {
        width: request.output.width,
        height: request.output.height,
    };
    let mut data = vec![0; dimensions.width as usize * dimensions.height as usize * 4];
    subject.resize_u8_rgba(
        ResizeInputU8Rgba {
            data: request.source.data,
            width: request.source.width,
            height: request.source.height,
            row_stride_elements: request.source.width as usize * 4,
        },
        ResizeOutputU8Rgba {
            data: &mut data,
            width: dimensions.width,
            height: dimensions.height,
            row_stride_elements: dimensions.width as usize * 4,
        },
        &params,
    )?;
    Ok(VerificationOutput {
        dimensions,
        pixels: Pixels::Rgba8 { data },
        warnings: Vec::new(),
    })
}

fn support_name(support: Support) -> &'static str {
    match support {
        Support::Fixed => "fixed",
        Support::ScaleAware => "scale-aware",
    }
}

fn dimensions<F: ImageFormat>(image: &ImageBuf<F>) -> Dimensions {
    Dimensions {
        width: image.dimensions().width(),
        height: image.dimensions().height(),
    }
}

fn packed<F: ImageFormat>(image: &ImageBuf<F>) -> Vec<F::Storage> {
    let view = image.as_view();
    (0..view.dimensions().height())
        .flat_map(|row| view.row(row).expect("validated logical row"))
        .copied()
        .collect()
}
