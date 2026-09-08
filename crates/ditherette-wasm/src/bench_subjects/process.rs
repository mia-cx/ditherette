//! Actual complete Process versus staged production calls, including durable boundary copies.

use super::{
    field_calls::{processor, NativeBoundary},
    reference::ReferenceRequest,
    verification::indexed_output,
    BenchSubject,
};
use crate::{
    image::contracts::IndexedImage,
    prod::{
        contract::request as prod,
        pipeline::{
            process::ProcessRequest,
            processor::{Processor, ResizeRequest},
            quantize::QuantizeRequest,
        },
    },
    spec::contract::request as spec,
};
use ditherette_bench_api::{
    verification::Operation, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};

pub const PROCESS_SUBJECT: &str = "prod:process:request:processor-v1";
pub const STAGED_SUBJECT: &str = "prod:process:request:staged-v1";

pub fn callable(id: &str) -> bool {
    matches!(id, PROCESS_SUBJECT | STAGED_SUBJECT)
}

/// Settings mapping and instance initialization precede samples, not processing allocations or copies.
pub struct CompleteCall<'a> {
    source: &'a [u8],
    request: ProcessRequest<'a>,
    staged: bool,
}

impl<'a> CompleteCall<'a> {
    pub fn new(input: &ReferenceRequest<'a>, subject: &str) -> Result<Self, BenchSubjectError> {
        input.dimensions()?;
        if !callable(subject) {
            return Err(BenchSubjectError::new("unknown complete Process subject"));
        }
        let ReferenceRequest::Processing(spec::Request::Process(input)) = *input else {
            return Err(BenchSubjectError::new(
                "Process subject requires a typed Process request",
            ));
        };
        // Both contract enums use the same frozen wire schema. Mapping stays outside every timed call.
        let recipe: prod::RecipeV1 =
            serde_json::from_value(serde_json::to_value(input.recipe).map_err(error)?)
                .map_err(error)?;
        Ok(Self {
            source: input.source.data,
            request: ProcessRequest {
                source_width: input.source.width,
                source_height: input.source.height,
                palette: input.palette,
                recipe,
            },
            staged: subject == STAGED_SUBJECT,
        })
    }

    pub fn output(&self, processor: &mut Processor) -> Result<IndexedImage, BenchSubjectError> {
        if !self.staged {
            return processor
                .process(self.request, &mut NativeBoundary(self.source))
                .map_err(error);
        }
        let recipe = self.request.recipe;
        let resized = processor
            .resize(
                ResizeRequest {
                    source_width: self.request.source_width,
                    source_height: self.request.source_height,
                    output: recipe.output,
                },
                &mut NativeBoundary(self.source),
            )
            .map_err(error)?;
        processor
            .dither_and_quantize(
                QuantizeRequest {
                    source_width: recipe.output.width,
                    source_height: recipe.output.height,
                    palette: self.request.palette,
                    alpha: recipe.alpha,
                    matching: recipe.matching,
                },
                recipe.dither,
                &mut NativeBoundary(resized.data()),
            )
            .map_err(error)
    }

    pub fn run(&self, processor: &mut Processor) -> Result<(), BenchSubjectError> {
        drop(std::hint::black_box(self.output(processor)?));
        Ok(())
    }
}

fn error(error: impl std::fmt::Debug) -> BenchSubjectError {
    BenchSubjectError::new(format!("{error:?}"))
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    [PROCESS_SUBJECT, STAGED_SUBJECT]
        .into_iter()
        .map(|id| {
            BenchSubject::Conformance(ConformanceBenchSubject {
                descriptor: SubjectDescriptor {
                    id: SubjectId::parse(id).expect("literal ID"),
                    display_name: id.into(),
                    source_file: "crates/ditherette-wasm/src/bench_subjects/process.rs".into(),
                    source_line: 1,
                    default_oracle: Some(
                        SubjectId::parse("spec:process:request:v1").expect("literal oracle"),
                    ),
                    capabilities: SubjectCapabilities {
                        pixel_formats: vec![PixelFormat::Indexed8],
                        supports_strided_io: false,
                        supports_tiling_params: false,
                        scalar_control: None,
                    },
                    params_schema: ParamSchema::default(),
                },
                operation: Operation::Process,
                run: if id == PROCESS_SUBJECT {
                    |r| {
                        Ok(indexed_output(
                            &CompleteCall::new(r, PROCESS_SUBJECT)?.output(&mut processor()?)?,
                        ))
                    }
                } else {
                    |r| {
                        Ok(indexed_output(
                            &CompleteCall::new(r, STAGED_SUBJECT)?.output(&mut processor()?)?,
                        ))
                    }
                },
            })
        })
        .collect()
}
