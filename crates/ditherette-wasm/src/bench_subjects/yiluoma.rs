//! Typed native Yliluoma call. Mapping and verification serialization remain outside samples.

use super::{
    fields::prod_placement, quantize::quantize_request, reference::ReferenceRequest,
    verification::indexed_output, BenchSubject,
};
use crate::{image::contracts::IndexedImage, prod, spec::contract::request as reference};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};
use prod::contract::request::{BayerSize, DitherPolicy, DitherQuantizeRequest};

pub const YLILUOMA_SUBJECT: &str = "prod:dither-and-quantize:yliluoma:literal-v1";
pub type YliluomaFn =
    for<'a> fn(DitherQuantizeRequest<'a>) -> Result<IndexedImage, BenchSubjectError>;

/// Select the actual production call, including validation, preparation, allocation, and result ownership.
pub fn yiluoma_function(id: &str) -> Option<YliluomaFn> {
    (id == YLILUOMA_SUBJECT).then_some(run)
}

fn run(request: DitherQuantizeRequest<'_>) -> Result<IndexedImage, BenchSubjectError> {
    prod::dither::yiluoma::dither_yiluoma(request, u64::MAX)
        .map_err(|error| BenchSubjectError::new(format!("{error:?}")))
}

/// Borrow and mechanically map the validated frozen contract before entering a timed call.
pub fn yiluoma_request<'a>(
    input: &ReferenceRequest<'a>,
) -> Result<DitherQuantizeRequest<'a>, BenchSubjectError> {
    input.dimensions()?;
    let ReferenceRequest::Processing(reference::Request::DitherAndQuantize(input)) = *input else {
        return Err(BenchSubjectError::new(
            "Yliluoma subject requires its typed dither request",
        ));
    };
    let reference::DitherPolicy::Yliluoma { size, placement } = input.dither else {
        return Err(BenchSubjectError::new(
            "Yliluoma subject rejects other dither families",
        ));
    };
    let size = match size {
        reference::BayerSize::Two => BayerSize::Two,
        reference::BayerSize::Four => BayerSize::Four,
        reference::BayerSize::Eight => BayerSize::Eight,
        reference::BayerSize::Sixteen => BayerSize::Sixteen,
    };
    Ok(DitherQuantizeRequest {
        quantize: quantize_request(&ReferenceRequest::Processing(reference::Request::Quantize(
            input.quantize,
        )))?,
        dither: DitherPolicy::Yliluoma {
            size,
            placement: prod_placement(placement),
        },
    })
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    vec![BenchSubject::Conformance(ConformanceBenchSubject {
        descriptor: SubjectDescriptor {
            id: SubjectId::parse(YLILUOMA_SUBJECT).expect("literal ID"),
            display_name: "scalar Yliluoma literal complete native call".into(),
            source_file: "crates/ditherette-wasm/src/prod/dither/yiluoma/request.rs".into(),
            source_line: 1,
            default_oracle: Some(
                SubjectId::parse("spec:dither-and-quantize:request:v1").expect("literal oracle"),
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
        run: |input| Ok(indexed_output(&run(yiluoma_request(input)?)?)),
    })]
}
