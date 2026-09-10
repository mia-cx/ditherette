//! Native complete-call boundaries. Input copies, durable result copies, and destruction stay in each call.

use super::{
    fields::prod_policy,
    quantize::quantize_request,
    reference::ReferenceRequest,
    verification::{indexed_output, rgba_output},
    BenchSubject,
};
use crate::{
    image::{contracts::IndexedImage, ImageBuf, ImageDimensions, PaletteIndex8, Rgba8},
    prod::{
        contract::{failure::Failure, request as prod},
        pipeline::{
            perturb::PerturbRequest,
            processor::{Boundary, Processor},
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
    spec::contract::request as spec,
};
use ditherette_bench_api::{
    verification::*, BenchSubjectError, ConformanceBenchSubject, ParamSchema, PixelFormat,
    SubjectCapabilities, SubjectDescriptor, SubjectId,
};

pub const PERTURB_SUBJECT: &str = "prod:perturb:request:processor-v1";
pub const SEPARABLE_SUBJECT: &str = "prod:dither-and-quantize:request:processor-v1";

/// Mapping and processor initialization precede samples. Each run uses the actual Processor method.
pub enum CompleteCall<'a> {
    Perturb {
        source: &'a [u8],
        request: PerturbRequest,
    },
    Separable {
        source: &'a [u8],
        request: QuantizeRequest<'a>,
        perturb: prod::PerturbPolicy,
    },
}

impl<'a> CompleteCall<'a> {
    pub fn new(request: &ReferenceRequest<'a>) -> Result<Self, BenchSubjectError> {
        request.dimensions()?;
        match *request {
            ReferenceRequest::Processing(spec::Request::Perturb(input)) => Ok(Self::Perturb {
                source: input.source.data,
                request: PerturbRequest {
                    source_width: input.source.width,
                    source_height: input.source.height,
                    perturb: prod_policy(input.perturb),
                },
            }),
            ReferenceRequest::Processing(spec::Request::DitherAndQuantize(input)) => {
                let spec::DitherPolicy::Separable { perturb } = input.dither else {
                    return Err(BenchSubjectError::new(
                        "field benchmark requires a separable dither request",
                    ));
                };
                let mapped = quantize_request(&ReferenceRequest::Processing(
                    spec::Request::Quantize(input.quantize),
                ))?;
                Ok(Self::Separable {
                    source: mapped.source.data,
                    request: QuantizeRequest {
                        source_width: mapped.source.width,
                        source_height: mapped.source.height,
                        palette: mapped.palette,
                        alpha: mapped.alpha,
                        matching: mapped.matching,
                    },
                    perturb: prod_policy(perturb),
                })
            }
            _ => Err(BenchSubjectError::new(
                "complete field subject requires its typed processing request",
            )),
        }
    }

    pub fn run(&self, processor: &mut Processor) -> Result<(), BenchSubjectError> {
        match *self {
            Self::Perturb { source, request } => {
                drop(std::hint::black_box(
                    processor
                        .perturb(request, &mut NativeBoundary(source))
                        .map_err(failure)?,
                ));
            }
            Self::Separable {
                source,
                request,
                perturb,
            } => {
                drop(std::hint::black_box(
                    processor
                        .dither_and_quantize(
                            request,
                            prod::DitherPolicy::Separable { perturb },
                            &mut NativeBoundary(source),
                        )
                        .map_err(failure)?,
                ));
            }
        }
        Ok(())
    }

    pub fn output(
        &self,
        processor: &mut Processor,
    ) -> Result<VerificationOutput, BenchSubjectError> {
        match *self {
            Self::Perturb { source, request } => Ok(rgba_output(
                &processor
                    .perturb(request, &mut NativeBoundary(source))
                    .map_err(failure)?,
            )),
            Self::Separable {
                source,
                request,
                perturb,
            } => Ok(indexed_output(
                &processor
                    .dither_and_quantize(
                        request,
                        prod::DitherPolicy::Separable { perturb },
                        &mut NativeBoundary(source),
                    )
                    .map_err(failure)?,
            )),
        }
    }
}

fn failure(error: Failure) -> BenchSubjectError {
    BenchSubjectError::new(format!("{error:?}"))
}

pub fn processor() -> Result<Processor, BenchSubjectError> {
    Processor::new(2 * 1024 * 1024 * 1024, 0).map_err(failure)
}

pub(super) struct NativeBoundary<'a>(pub(super) &'a [u8]);
impl Boundary for NativeBoundary<'_> {
    type Output = ImageBuf<Rgba8>;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        if compare && destination == self.0 {
            return Ok(true);
        }
        Boundary::copy_input(self, destination)?;
        Ok(false)
    }
    fn complete(
        &mut self,
        bytes: &[u8],
        dimensions: ImageDimensions,
    ) -> Result<Self::Output, Failure> {
        Ok(ImageBuf::from_vec_packed(bytes.to_vec(), dimensions).expect("validated output"))
    }
}
impl QuantizeBoundary for NativeBoundary<'_> {
    type Output = IndexedImage;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        Boundary::snapshot_input(self, destination, compare)
    }
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, Failure> {
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices.to_vec(), dimensions)
                .expect("validated indices"),
            palette: palette.palette.clone(),
            warnings: palette.warnings.to_vec(),
        })
    }
}

fn output(
    request: &ReferenceRequest<'_>,
    expected: Operation,
) -> Result<VerificationOutput, BenchSubjectError> {
    if request.semantics().operation != expected {
        return Err(BenchSubjectError::new(
            "complete subject and operation differ",
        ));
    }
    CompleteCall::new(request)?.output(&mut processor()?)
}

pub(super) fn subjects() -> Vec<BenchSubject> {
    use super::reference::ReferenceFn;
    let entries: [(&str, &str, Operation, PixelFormat, ReferenceFn); 2] = [
        (
            PERTURB_SUBJECT,
            "spec:perturb:request:v1",
            Operation::Perturb,
            PixelFormat::Rgba8,
            |r| output(r, Operation::Perturb),
        ),
        (
            SEPARABLE_SUBJECT,
            "spec:dither-and-quantize:request:v1",
            Operation::DitherAndQuantize,
            PixelFormat::Indexed8,
            |r| output(r, Operation::DitherAndQuantize),
        ),
    ];
    entries
        .into_iter()
        .map(|(id, oracle, operation, format, run)| {
            BenchSubject::Conformance(ConformanceBenchSubject {
                descriptor: SubjectDescriptor {
                    id: SubjectId::parse(id).expect("literal ID"),
                    display_name: id.into(),
                    source_file: "crates/ditherette-wasm/src/bench_subjects/field_calls.rs".into(),
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
                run,
            })
        })
        .collect()
}
