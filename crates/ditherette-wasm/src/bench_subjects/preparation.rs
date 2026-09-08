//! Development-only ordinary Processor subjects. Mapping stays outside call timers.

use super::{
    field_calls::{processor, NativeBoundary},
    process, quantize,
    reference::ReferenceRequest,
    verification::{indexed_output, rgba_output},
    BenchSubject,
};
use crate::{
    image::{contracts::IndexedImage, ImageBuf, Rgba8},
    prod::{
        contract::request as prod,
        pipeline::{
            processor::{Processor, ResizeRequest},
            quantize::QuantizeRequest,
        },
    },
    spec::contract::request as spec,
};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};

pub const RESIZE_SUBJECT: &str = "prod:resize:request:processor-v1";
pub const QUANTIZE_SUBJECT: &str = "prod:quantize:request:processor-v1";

pub enum CompleteCall<'a> {
    Resize(ResizeRequest),
    Quantize(QuantizeRequest<'a>),
    Process(process::CompleteCall<'a>),
}

pub enum Output {
    Rgba(ImageBuf<Rgba8>),
    Indexed(IndexedImage),
}

impl Output {
    /// Convert only untimed results into transport evidence.
    pub fn verification(&self) -> VerificationOutput {
        match self {
            Self::Rgba(image) => rgba_output(image),
            Self::Indexed(image) => indexed_output(image),
        }
    }
}

impl<'a> CompleteCall<'a> {
    pub fn new(request: &ReferenceRequest<'a>) -> Result<Self, BenchSubjectError> {
        request.dimensions()?;
        match *request {
            ReferenceRequest::Processing(spec::Request::Resize(input)) => {
                let output: prod::Output =
                    serde_json::from_value(serde_json::to_value(input.output).map_err(error)?)
                        .map_err(error)?;
                Ok(Self::Resize(ResizeRequest {
                    source_width: input.source.width,
                    source_height: input.source.height,
                    output,
                }))
            }
            ReferenceRequest::Processing(spec::Request::Quantize(_)) => {
                let input = quantize::quantize_request(request)?;
                Ok(Self::Quantize(QuantizeRequest {
                    source_width: input.source.width,
                    source_height: input.source.height,
                    palette: input.palette,
                    alpha: input.alpha,
                    matching: input.matching,
                }))
            }
            ReferenceRequest::Processing(spec::Request::Process(_)) => Ok(Self::Process(
                process::CompleteCall::new(request, process::PROCESS_SUBJECT)?,
            )),
            _ => Err(BenchSubjectError::new(
                "preparation subject requires resize, quantize, or process",
            )),
        }
    }

    /// Includes ordinary key preparation, input and result copies. The caller owns result destruction.
    pub fn output(
        &self,
        processor: &mut Processor,
        source: &[u8],
    ) -> Result<Output, BenchSubjectError> {
        match self {
            Self::Resize(request) => processor
                .resize(*request, &mut NativeBoundary(source))
                .map(Output::Rgba)
                .map_err(error),
            Self::Quantize(request) => processor
                .quantize(*request, &mut NativeBoundary(source))
                .map(Output::Indexed)
                .map_err(error),
            Self::Process(call) => call
                .output_with_source(processor, source)
                .map(Output::Indexed),
        }
    }
}

fn error(error: impl std::fmt::Debug) -> BenchSubjectError {
    BenchSubjectError::new(format!("{error:?}"))
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    [
        (
            RESIZE_SUBJECT,
            "spec:resize:request:v1",
            Operation::Resize,
            PixelFormat::Rgba8,
        ),
        (
            QUANTIZE_SUBJECT,
            "spec:quantize:request:v1",
            Operation::Quantize,
            PixelFormat::Indexed8,
        ),
    ]
    .into_iter()
    .map(|(id, oracle, operation, format)| {
        BenchSubject::Conformance(ConformanceBenchSubject {
            descriptor: SubjectDescriptor {
                id: SubjectId::parse(id).expect("literal ID"),
                display_name: id.into(),
                source_file: "crates/ditherette-wasm/src/bench_subjects/preparation.rs".into(),
                source_line: 1,
                default_oracle: Some(SubjectId::parse(oracle).expect("literal ID")),
                capabilities: SubjectCapabilities {
                    pixel_formats: vec![format],
                    supports_strided_io: false,
                    supports_tiling_params: false,
                    scalar_control: None,
                },
                params_schema: ParamSchema::default(),
            },
            operation,
            run: |request| {
                Ok(CompleteCall::new(request)?
                    .output(&mut processor()?, request.source().data)?
                    .verification())
            },
        })
    })
    .collect()
}
