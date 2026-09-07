//! Spec resize algorithms and coordinate semantics.
//!
//! Resize specs favor direct coordinate mapping and obvious loops. They are the
//! byte-for-byte oracles that future production scalar/SIMD/tiled resize paths
//! must match for exact modes.

pub mod common;
pub mod scalar;

use crate::{
    image::{contracts::Rgba8Image, ImageBuf},
    spec::contract::{
        error::DitheretteError,
        request::{Anchor, Request, ResizePolicy, ResizeRequest, Support},
    },
};
use common::alignment::ResizeAnchor;
use scalar::convolution::SupportPolicy;

/// Validates and executes one complete reference resize, returning owned RGBA8.
/// Channels are sampled independently. Alpha preparation belongs to quantization.
pub fn resize(request: ResizeRequest<'_>) -> Result<Rgba8Image, DitheretteError> {
    let layout = Request::Resize(request).validate()?;
    let mut image = ImageBuf::new_packed(layout.output).expect("validated output storage length");
    let source = layout.source;
    let output = image.as_view_mut();
    match request.output.resize {
        ResizePolicy::Nearest { anchor } => {
            scalar::nearest::resize_nearest_into(source, output, reference_anchor(anchor))
        }
        ResizePolicy::Area {} => scalar::area::resize_area_into(source, output),
        ResizePolicy::Bilinear { anchor } => {
            scalar::bilinear::resize_bilinear_into(source, output, reference_anchor(anchor))
        }
        ResizePolicy::Bicubic { anchor, support } => scalar::bicubic::resize_bicubic_into(
            source,
            output,
            reference_anchor(anchor),
            reference_support(support),
        ),
        ResizePolicy::Lanczos2 { anchor, support } => scalar::lanczos::resize_lanczos2_into(
            source,
            output,
            reference_anchor(anchor),
            reference_support(support),
        ),
        ResizePolicy::Lanczos3 { anchor, support } => scalar::lanczos::resize_lanczos3_into(
            source,
            output,
            reference_anchor(anchor),
            reference_support(support),
        ),
        ResizePolicy::Trilinear { anchor } => {
            scalar::trilinear::resize_trilinear_into(source, output, reference_anchor(anchor))
        }
    }
    Ok(image)
}

fn reference_anchor(anchor: Anchor) -> ResizeAnchor {
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

fn reference_support(support: Support) -> SupportPolicy {
    match support {
        Support::Fixed => SupportPolicy::Fixed,
        Support::ScaleAware => SupportPolicy::ScaleAware,
    }
}
