//! Native diffusion call registration. Contract mapping stays outside measured calls.

use super::{
    quantize::quantize_request, reference::ReferenceRequest, verification::indexed_output,
    BenchSubject,
};
use crate::{
    image::contracts::IndexedImage,
    prod::{contract::request as prod, dither::error_diffusion},
    spec::contract::request as spec,
};
use ditherette_bench_api::{
    verification::Operation, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};

pub const SUBJECT: &str = "prod:dither-and-quantize:diffusion:full-image-v1";
pub type DiffusionFn =
    for<'a> fn(prod::DitherQuantizeRequest<'a>) -> Result<IndexedImage, BenchSubjectError>;

/// Includes full-image work, palette preparation, and owned result allocation.
pub fn function(id: &str) -> Option<DiffusionFn> {
    (id == SUBJECT).then_some(|request| {
        error_diffusion::diffuse(request).map_err(|error| BenchSubjectError::new(error.to_string()))
    })
}

pub fn request<'a>(
    input: &ReferenceRequest<'a>,
) -> Result<prod::DitherQuantizeRequest<'a>, BenchSubjectError> {
    let ReferenceRequest::Processing(spec::Request::DitherAndQuantize(input)) = *input else {
        return Err(BenchSubjectError::new(
            "diffusion subject requires a dither-and-quantize request",
        ));
    };
    if !matches!(input.dither, spec::DitherPolicy::Diffusion { .. }) {
        return Err(BenchSubjectError::new(
            "diffusion subject requires a diffusion recipe",
        ));
    }
    Ok(prod::DitherQuantizeRequest {
        quantize: quantize_request(&ReferenceRequest::Processing(spec::Request::Quantize(
            input.quantize,
        )))?,
        dither: serde_json::from_value(
            serde_json::to_value(input.dither)
                .map_err(|error| BenchSubjectError::new(error.to_string()))?,
        )
        .map_err(|error| BenchSubjectError::new(error.to_string()))?,
    })
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    vec![BenchSubject::Conformance(ConformanceBenchSubject {
        descriptor: SubjectDescriptor {
            id: SubjectId::parse(SUBJECT).expect("literal ID"),
            display_name: "literal full-image diffusion complete native call".into(),
            source_file: "crates/ditherette-wasm/src/prod/dither/error_diffusion.rs".into(),
            source_line: 1,
            default_oracle: Some(
                SubjectId::parse("spec:dither-and-quantize:request:v1").expect("literal ID"),
            ),
            capabilities: SubjectCapabilities {
                pixel_formats: vec![PixelFormat::Indexed8],
                supports_strided_io: false,
                supports_tiling_params: false,
                scalar_control: None,
            },
            params_schema: ParamSchema::default(),
        },
        operation: Operation::DitherAndQuantize,
        run: |input| {
            Ok(indexed_output(&function(SUBJECT)
                .expect("registered callable")(
                request(input)?
            )?))
        },
    })]
}
