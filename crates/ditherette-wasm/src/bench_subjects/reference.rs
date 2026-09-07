//! Callable semantic references in the existing benchmark registry.
//! Concrete processing requests remain the public contract's borrowed types.

use super::{
    verification::{color_space, indexed_output, reference_resize},
    BenchSubject,
};
use crate::spec::{self, contract::request::*};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamField, ParamSchema,
    PixelFormat, SubjectCapabilities, SubjectDescriptor, SubjectId,
};
use serde::{Serialize, Serializer};

/// Typed processing requests plus the internal packed-f32 color conversion operation.
#[derive(Debug, Clone, Copy)]
pub enum ReferenceRequest<'a> {
    Processing(Request<'a>),
    Color {
        source: Source<'a>,
        space: WorkingSpace,
    },
}

/// Callable registry adapter compatible with S05's generic VerificationSubject.
pub type ReferenceFn =
    for<'a> fn(&ReferenceRequest<'a>) -> Result<VerificationOutput, BenchSubjectError>;

impl ReferenceRequest<'_> {
    /// Caller-owned source used by the benchmark's complete input-content digest.
    pub fn source(&self) -> Source<'_> {
        match *self {
            Self::Processing(Request::Process(request)) => request.source,
            Self::Processing(Request::Resize(request)) => request.source,
            Self::Processing(Request::Perturb(request)) => request.source,
            Self::Processing(Request::Quantize(request)) => request.source,
            Self::Processing(Request::DitherAndQuantize(request)) => request.quantize.source,
            Self::Color { source, .. } => source,
        }
    }

    /// Validates the concrete contract before identities or outputs are published.
    pub fn dimensions(&self) -> Result<Dimensions, BenchSubjectError> {
        let output = match *self {
            Self::Processing(request) => request.validate().map(|layout| layout.output),
            Self::Color { source, .. } => Request::Resize(ResizeRequest {
                version: 1,
                source,
                output: Output {
                    width: source.width,
                    height: source.height,
                    resize: ResizePolicy::Nearest {
                        anchor: Anchor::Center,
                    },
                },
            })
            .validate()
            .map(|layout| layout.output),
        }
        .map_err(|error| BenchSubjectError::new(error.to_string()))?;
        Ok(Dimensions {
            width: output.width(),
            height: output.height(),
        })
    }

    /// Primary space identifies color coordinates or matching. Serialization binds every stage.
    pub fn semantics(&self) -> SemanticIdentity {
        let (operation, recipe, version, space) = match *self {
            Self::Processing(Request::Process(request)) => (
                Operation::Process,
                "public-process",
                request.recipe.version,
                Some(request.recipe.matching.space()),
            ),
            Self::Processing(Request::Resize(request)) => {
                (Operation::Resize, "public-resize", request.version, None)
            }
            Self::Processing(Request::Perturb(request)) => (
                Operation::Perturb,
                "public-perturb",
                request.version,
                Some(request.perturb.space),
            ),
            Self::Processing(Request::Quantize(request)) => (
                Operation::Quantize,
                "public-quantize",
                request.version,
                Some(request.matching.space()),
            ),
            Self::Processing(Request::DitherAndQuantize(request)) => (
                Operation::DitherAndQuantize,
                "public-dither-and-quantize",
                request.quantize.version,
                Some(request.quantize.matching.space()),
            ),
            Self::Color { space, .. } => (Operation::Color, "color-f32-roundtrip", 1, Some(space)),
        };
        SemanticIdentity {
            operation,
            recipe: recipe.into(),
            version,
            space: space.map(color_space),
        }
    }
}

/// Serialize normalized settings only. Source bytes/dimensions use the separate input digest.
/// This includes palette order, all recipe fields, and distinct perturb/matching spaces.
impl Serialize for ReferenceRequest<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Processing(Request::Process(request)) => {
                ("process", request.palette, request.recipe).serialize(serializer)
            }
            Self::Processing(Request::Resize(request)) => {
                ("resize", request.version, request.output).serialize(serializer)
            }
            Self::Processing(Request::Perturb(request)) => {
                ("perturb", request.version, request.perturb).serialize(serializer)
            }
            Self::Processing(Request::Quantize(request)) => (
                "quantize",
                request.version,
                request.palette,
                request.alpha,
                request.matching,
            )
                .serialize(serializer),
            Self::Processing(Request::DitherAndQuantize(request)) => (
                "dither-and-quantize",
                request.quantize.version,
                request.quantize.palette,
                request.quantize.alpha,
                request.quantize.matching,
                request.dither,
            )
                .serialize(serializer),
            Self::Color { space, .. } => ("color-f32-roundtrip", 1u32, space).serialize(serializer),
        }
    }
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    let mut subjects = vec![
        subject(
            "resize",
            Operation::Resize,
            PixelFormat::Rgba8,
            "resize/mod.rs",
            |request| match request {
                ReferenceRequest::Processing(Request::Resize(request)) => reference_resize(request),
                _ => Err(wrong_request("resize")),
            },
        ),
        subject(
            "quantize",
            Operation::Quantize,
            PixelFormat::Indexed8,
            "quantize/mod.rs",
            |request| match request {
                ReferenceRequest::Processing(Request::Quantize(request)) => {
                    spec::quantize::quantize(*request)
                        .map(|image| indexed_output(&image))
                        .map_err(|error| BenchSubjectError::new(error.to_string()))
                }
                _ => Err(wrong_request("quantize")),
            },
        ),
    ];
    macro_rules! color_adapter {
        ($space:ident) => {
            |request| color(request, WorkingSpace::$space)
        };
    }
    let colors: [(&str, &str, ReferenceFn); 7] = [
        ("srgb", "srgb.rs", color_adapter!(Srgb)),
        ("linear-rgb", "linear.rs", color_adapter!(LinearRgb)),
        ("oklab", "oklab.rs", color_adapter!(Oklab)),
        ("oklch", "oklch.rs", color_adapter!(Oklch)),
        ("cielab", "cielab.rs", color_adapter!(Cielab)),
        ("cielch", "cielch.rs", color_adapter!(Cielch)),
        ("ycbcr", "ycbcr.rs", color_adapter!(Ycbcr)),
    ];
    for (name, file, run) in colors {
        let mut entry = subject(
            "color",
            Operation::Color,
            PixelFormat::Color32,
            &format!("color/{file}"),
            run,
        );
        if let BenchSubject::Conformance(subject) = &mut entry {
            subject.descriptor.id = SubjectId::parse(format!("spec:color:{name}:f32-roundtrip-v1"))
                .expect("literal subject ID");
            subject.descriptor.display_name =
                format!("reference {name} packed-f32 conversion and inverse");
        }
        subjects.push(entry);
    }
    subjects
}

fn subject(
    domain: &str,
    operation: Operation,
    format: PixelFormat,
    file: &str,
    run: ReferenceFn,
) -> BenchSubject {
    BenchSubject::Conformance(ConformanceBenchSubject {
        descriptor: SubjectDescriptor {
            id: SubjectId::parse(format!("spec:{domain}:request:v1")).expect("literal subject ID"),
            display_name: format!("reference {domain} complete typed request"),
            source_file: format!("crates/ditherette-wasm/src/spec/{file}"), source_line: 1, default_oracle: None,
            capabilities: SubjectCapabilities { pixel_formats: vec![format], supports_strided_io: false,
                supports_tiling_params: false, scalar_control: None },
            params_schema: ParamSchema { fields: vec![ParamField { name: "request".into(),
                description: "Typed reference request; normalized settings retain every supported mode and stage.".into() }] },
        }, operation, run,
    })
}

fn wrong_request(method: &str) -> BenchSubjectError {
    BenchSubjectError::new(format!(
        "reference {method} subject requires its matching typed request"
    ))
}

fn color(
    request: &ReferenceRequest<'_>,
    expected: WorkingSpace,
) -> Result<VerificationOutput, BenchSubjectError> {
    let ReferenceRequest::Color { source, space } = *request else {
        return Err(wrong_request("color"));
    };
    if space != expected {
        return Err(BenchSubjectError::new(
            "color subject and requested working space differ",
        ));
    }
    let dimensions = request.dimensions()?;
    let mut coordinates = Vec::with_capacity(source.data.len() / 4 * 3);
    let mut alpha = Vec::with_capacity(source.data.len() / 4);
    let mut rendered_rgba = Vec::with_capacity(source.data.len());
    for pixel in source.data.chunks_exact(4) {
        let converted = spec::color::rgb8_to_coordinates([pixel[0], pixel[1], pixel[2]], space);
        coordinates.extend_from_slice(&converted);
        alpha.push(pixel[3]);
        rendered_rgba.extend_from_slice(&spec::color::coordinates_to_rgb8(converted, space));
        rendered_rgba.push(pixel[3]);
    }
    Ok(VerificationOutput {
        dimensions,
        pixels: Pixels::Color {
            space: color_space(space),
            coordinates,
            alpha,
            rendered_rgba: Some(rendered_rgba),
        },
        warnings: Vec::new(),
    })
}
