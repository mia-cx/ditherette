//! Native quantize and packed forward-conversion adapters. No timing occurs here.
//! The baseline artifact replaces only the quantize wrapper and its subject ID.

use super::{
    reference::ReferenceRequest,
    verification::{color_space, indexed_output},
    BenchSubject,
};
use crate::{
    image::{contracts::IndexedImage, ImageDimensions, ImageView, Rgba8},
    prod::{
        self,
        color::packed::{Converter, OrdinarySpace},
        contract::request,
    },
    spec::contract::request as reference,
};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};

pub const QUANTIZE_SUBJECT: &str = "baseline:quantize:request:literal";
pub type QuantizeFn =
    for<'a> fn(request::QuantizeRequest<'a>) -> Result<IndexedImage, BenchSubjectError>;

/// The selected full-call wrapper includes all owned preparation and result allocation.
pub fn quantize_function(id: &str) -> Option<QuantizeFn> {
    (id == QUANTIZE_SUBJECT).then_some(quantize)
}

fn quantize(request: request::QuantizeRequest<'_>) -> Result<IndexedImage, BenchSubjectError> {
    prod::quantize::quantize(request).map_err(|error| BenchSubjectError::new(format!("{error:?}")))
}

/// Mechanical contract mapping stays outside the timed production call.
pub fn quantize_request<'a>(
    input: &ReferenceRequest<'a>,
) -> Result<request::QuantizeRequest<'a>, BenchSubjectError> {
    let ReferenceRequest::Processing(reference::Request::Quantize(input)) = *input else {
        return Err(BenchSubjectError::new(
            "quantize subject requires its typed request",
        ));
    };
    let matching = match input.matching {
        reference::MatchPolicy::SrgbEuclidean => request::MatchPolicy::SrgbEuclidean,
        reference::MatchPolicy::LinearRgbEuclidean => request::MatchPolicy::LinearRgbEuclidean,
        reference::MatchPolicy::OklabEuclidean => request::MatchPolicy::OklabEuclidean,
        reference::MatchPolicy::CielabEuclidean => request::MatchPolicy::CielabEuclidean,
        reference::MatchPolicy::YcbcrEuclidean => request::MatchPolicy::YcbcrEuclidean,
        _ => {
            return Err(BenchSubjectError::new(
                "quantize adapter supports ordinary Euclidean spaces only",
            ))
        }
    };
    Ok(request::QuantizeRequest {
        version: input.version,
        source: request::Source {
            width: input.source.width,
            height: input.source.height,
            data: input.source.data,
        },
        palette: input.palette,
        matching,
        alpha: match input.alpha {
            reference::AlphaPolicy::Preserve { threshold } => {
                request::AlphaPolicy::Preserve { threshold }
            }
            reference::AlphaPolicy::Premultiplied {} => request::AlphaPolicy::Premultiplied {},
            reference::AlphaPolicy::Matte { rgb } => request::AlphaPolicy::Matte { rgb },
        },
    })
}

pub fn ordinary_space(space: reference::WorkingSpace) -> Result<OrdinarySpace, BenchSubjectError> {
    use reference::WorkingSpace::*;
    match space {
        Srgb => Ok(OrdinarySpace::Srgb),
        LinearRgb => Ok(OrdinarySpace::LinearRgb),
        Oklab => Ok(OrdinarySpace::Oklab),
        Cielab => Ok(OrdinarySpace::Cielab),
        Ycbcr => Ok(OrdinarySpace::Ycbcr),
        _ => Err(BenchSubjectError::new(
            "forward adapter requires an ordinary space",
        )),
    }
}

pub fn color_subject(space: reference::WorkingSpace) -> Result<&'static str, BenchSubjectError> {
    use reference::WorkingSpace::*;
    match space {
        Srgb => Ok("prod:color:srgb:packed-forward"),
        LinearRgb => Ok("prod:color:linear-rgb:packed-forward"),
        Oklab => Ok("prod:color:oklab:packed-forward"),
        Cielab => Ok("prod:color:cielab:packed-forward"),
        Ycbcr => Ok("prod:color:ycbcr:packed-forward"),
        _ => Err(BenchSubjectError::new(
            "forward adapter requires an ordinary space",
        )),
    }
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    let mut subjects = vec![subject(
        QUANTIZE_SUBJECT,
        "spec:quantize:request:v1",
        Operation::Quantize,
        PixelFormat::Indexed8,
        "crates/ditherette-wasm/src/prod/quantize/mod.rs",
        |input| Ok(indexed_output(&quantize(quantize_request(input)?)?)),
    )];
    use reference::WorkingSpace::*;
    let spaces: [(reference::WorkingSpace, &str, super::reference::ReferenceFn); 5] = [
        (Srgb, "srgb", |r| color(r, Srgb)),
        (LinearRgb, "linear-rgb", |r| color(r, LinearRgb)),
        (Oklab, "oklab", |r| color(r, Oklab)),
        (Cielab, "cielab", |r| color(r, Cielab)),
        (Ycbcr, "ycbcr", |r| color(r, Ycbcr)),
    ];
    for (space, name, run) in spaces {
        subjects.push(subject(
            color_subject(space).expect("ordinary space"),
            &format!("spec:color:{name}:f32-roundtrip-v1"),
            Operation::Color,
            PixelFormat::Color32,
            "crates/ditherette-wasm/src/prod/color/packed.rs",
            run,
        ));
    }
    subjects
}

fn subject(
    id: &str,
    oracle: &str,
    operation: Operation,
    format: PixelFormat,
    source: &str,
    run: super::reference::ReferenceFn,
) -> BenchSubject {
    BenchSubject::Conformance(ConformanceBenchSubject {
        descriptor: SubjectDescriptor {
            id: SubjectId::parse(id).expect("literal ID"),
            display_name: id.into(),
            source_file: source.into(),
            source_line: 1,
            default_oracle: Some(SubjectId::parse(oracle).expect("literal oracle")),
            capabilities: SubjectCapabilities {
                pixel_formats: vec![format],
                supports_strided_io: false,
                supports_tiling_params: false,
                scalar_control: None,
            },
            params_schema: ParamSchema::default(),
        },
        operation,
        run,
    })
}

fn color(
    input: &ReferenceRequest<'_>,
    expected: reference::WorkingSpace,
) -> Result<VerificationOutput, BenchSubjectError> {
    let ReferenceRequest::Color { source, space } = *input else {
        return Err(BenchSubjectError::new(
            "forward subject requires a color request",
        ));
    };
    if space != expected {
        return Err(BenchSubjectError::new(
            "forward subject and requested space differ",
        ));
    }
    let dimensions = input.dimensions()?;
    let view = ImageView::<Rgba8>::packed(
        source.data,
        ImageDimensions::new(source.width, source.height)
            .map_err(|e| BenchSubjectError::new(e.to_string()))?,
    )
    .map_err(|e| BenchSubjectError::new(e.to_string()))?;
    let mut coordinates = vec![0.0; source.data.len() / 4 * 3];
    Converter::new(ordinary_space(space)?).rgba8_into(view, &mut coordinates);
    // A common frozen inverse renders diagnostic bytes only. The exact gate checks actual coordinates.
    let mut rendered_rgba = Vec::with_capacity(source.data.len());
    let alpha: Vec<_> = source.data.chunks_exact(4).map(|p| p[3]).collect();
    for (coordinates, alpha) in coordinates.chunks_exact(3).zip(&alpha) {
        rendered_rgba.extend_from_slice(&crate::spec::color::coordinates_to_rgb8(
            [coordinates[0], coordinates[1], coordinates[2]],
            space,
        ));
        rendered_rgba.push(*alpha);
    }
    Ok(VerificationOutput {
        dimensions,
        pixels: Pixels::Color {
            space: color_space(space),
            coordinates,
            alpha,
            rendered_rgba: Some(rendered_rgba),
        },
        warnings: vec![],
    })
}
