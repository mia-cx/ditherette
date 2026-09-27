//! Coverage-weighted resize: filters colour premultiplied by alpha.
//!
//! The frozen v1 kernels run unchanged on a four-channel `f32` carrier that holds
//! premultiplied colour and alpha. Dividing the filtered colour by the filtered
//! alpha weights every tap's colour by its coverage, so hidden colour under
//! transparent pixels never reaches the output.

use crate::{
    image::{contracts::Rgba8Image, ImageBuf, ImageDimensions, ImageView, LinearRgba32, Rgba8},
    spec::{
        contract::{
            error::DitheretteError,
            request::{Anchor, Request, ResizePolicy, ResizeRequest, Support},
        },
        resize::{
            self,
            common::{
                alignment::ResizeAnchor, coordinates::map_axis_position, sample::ResizeSample,
            },
            scalar::{self, convolution::SupportPolicy},
        },
    },
};

const OPAQUE: u8 = 255;
const BYTE_MAX: f64 = 255.0;

/// Resizes like v1 `resize`, except that filtering kernels weight colour by coverage.
///
/// Nearest and fully opaque sources return exactly the v1 result. Other sources
/// filter premultiplied colour, then divide by the filtered alpha. Kernels with
/// negative lobes also clamp alpha to the range of their main lobe, so ringing
/// cannot invent coverage in transparent areas.
pub fn resize(request: ResizeRequest<'_>) -> Result<Rgba8Image, DitheretteError> {
    let layout = Request::Resize(request).validate()?;
    let policy = request.output.resize;
    if matches!(policy, ResizePolicy::Nearest { .. }) || is_opaque(layout.source) {
        return resize::resize(request);
    }
    let carrier = premultiply(layout.source);
    let mut filtered = ImageBuf::<LinearRgba32>::new_packed(layout.output)
        .expect("validated output storage length");
    let source = carrier.as_view();
    let output = filtered.as_view_mut();
    let bounds = match policy {
        ResizePolicy::Nearest { .. } => unreachable!("nearest returned the v1 result"),
        ResizePolicy::Area {} => {
            scalar::area::resize_area_into(source, output);
            None
        }
        ResizePolicy::Bilinear { anchor } => {
            scalar::bilinear::resize_bilinear_into(source, output, reference_anchor(anchor));
            None
        }
        ResizePolicy::Trilinear { anchor } => {
            scalar::trilinear::resize_trilinear_into(source, output, reference_anchor(anchor));
            None
        }
        ResizePolicy::Bicubic { anchor, support } => {
            let (anchor, support) = (reference_anchor(anchor), reference_support(support));
            scalar::bicubic::resize_bicubic_into(source, output, anchor, support);
            Some(main_lobe_alpha(
                layout.source,
                layout.output,
                anchor,
                support,
            ))
        }
        ResizePolicy::Lanczos2 { anchor, support } => {
            let (anchor, support) = (reference_anchor(anchor), reference_support(support));
            scalar::lanczos::resize_lanczos2_into(source, output, anchor, support);
            Some(main_lobe_alpha(
                layout.source,
                layout.output,
                anchor,
                support,
            ))
        }
        ResizePolicy::Lanczos3 { anchor, support } => {
            let (anchor, support) = (reference_anchor(anchor), reference_support(support));
            scalar::lanczos::resize_lanczos3_into(source, output, anchor, support);
            Some(main_lobe_alpha(
                layout.source,
                layout.output,
                anchor,
                support,
            ))
        }
    };
    Ok(unpremultiply(&filtered, bounds.as_deref()))
}

/// True when every source pixel is fully opaque.
pub fn is_opaque(source: ImageView<'_, Rgba8>) -> bool {
    (0..source.dimensions().height()).all(|y| {
        source
            .row(y)
            .expect("source y from dimensions")
            .chunks_exact(4)
            .all(|pixel| pixel[3] == OPAQUE)
    })
}

/// Colour times coverage, in byte units, beside the unchanged alpha.
pub fn premultiply(source: ImageView<'_, Rgba8>) -> ImageBuf<LinearRgba32> {
    let mut carrier =
        ImageBuf::<LinearRgba32>::new_packed(source.dimensions()).expect("source storage length");
    let width = source.dimensions().width() as usize;
    for y in 0..source.dimensions().height() {
        let row = source.row(y).expect("source y from dimensions");
        let start = y as usize * width * 4;
        let target = &mut carrier.data_mut()[start..start + width * 4];
        for (pixel, out) in row.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
            let coverage = f64::from(pixel[3]) / BYTE_MAX;
            for channel in 0..3 {
                out[channel] = (f64::from(pixel[channel]) * coverage) as f32;
            }
            out[3] = f32::from(pixel[3]);
        }
    }
    carrier
}

/// Divides filtered colour by filtered alpha and rounds to bytes.
///
/// `bounds` holds each output pixel's `(min, max)` main-lobe source alpha; alpha is clamped
/// to it, while colour still divides by the unclamped alpha it was filtered with.
/// Fully transparent results store black, and so does colour whose filtered alpha is not positive.
pub fn unpremultiply(filtered: &ImageBuf<LinearRgba32>, bounds: Option<&[(u8, u8)]>) -> Rgba8Image {
    let mut output =
        Rgba8Image::new_packed(filtered.dimensions()).expect("filtered storage length");
    let pixels = filtered
        .data()
        .chunks_exact(4)
        .zip(output.data_mut().chunks_exact_mut(4));
    for (index, (pixel, out)) in pixels.enumerate() {
        let alpha = f64::from(pixel[3]);
        let clamped = bounds.map_or(alpha, |bounds| {
            let (low, high) = bounds[index];
            alpha.clamp(f64::from(low), f64::from(high))
        });
        out[3] = u8::from_f64(clamped);
        if out[3] == 0 || alpha <= 0.0 {
            out[..3].fill(0);
            continue;
        }
        for channel in 0..3 {
            out[channel] = u8::from_f64(f64::from(pixel[channel]) * BYTE_MAX / alpha);
        }
    }
    output
}

/// Per output pixel, the lowest and highest source alpha among the taps of the kernel's
/// main lobe: distances below one kernel unit on both axes, with the convolution's scale,
/// coordinate mapping, and edge clamping.
pub fn main_lobe_alpha(
    source: ImageView<'_, Rgba8>,
    output: ImageDimensions,
    anchor: ResizeAnchor,
    support: SupportPolicy,
) -> Vec<(u8, u8)> {
    let (width, height) = (source.dimensions().width(), source.dimensions().height());
    let (x_alignment, y_alignment) = anchor.axes();
    let x_scale = axis_scale(width, output.width(), support);
    let y_scale = axis_scale(height, output.height(), support);
    let mut bounds = Vec::with_capacity(output.width() as usize * output.height() as usize);
    for output_y in 0..output.height() {
        let y_position = map_axis_position(output_y, height, output.height(), y_alignment);
        let rows = lobe_taps(y_position, y_scale, height);
        for output_x in 0..output.width() {
            let x_position = map_axis_position(output_x, width, output.width(), x_alignment);
            let columns = lobe_taps(x_position, x_scale, width);
            let mut range = (u8::MAX, u8::MIN);
            for &y in &rows {
                let row = source.row(y).expect("clamped source y");
                for &x in &columns {
                    let alpha = row[x as usize * 4 + 3];
                    range = (range.0.min(alpha), range.1.max(alpha));
                }
            }
            bounds.push(range);
        }
    }
    bounds
}

/// Clamped source indices strictly within one kernel unit of `position`.
fn lobe_taps(position: f64, scale: f64, len: u32) -> Vec<u32> {
    let first = (position - scale).floor() as i64;
    let last = (position + scale).ceil() as i64;
    (first..=last)
        .filter(|&tap| ((tap as f64 - position) / scale).abs() < 1.0)
        .map(|tap| tap.clamp(0, i64::from(len) - 1) as u32)
        .collect()
}

fn axis_scale(source_len: u32, output_len: u32, support: SupportPolicy) -> f64 {
    match support {
        SupportPolicy::Fixed => 1.0,
        SupportPolicy::ScaleAware => (f64::from(source_len) / f64::from(output_len)).max(1.0),
    }
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
