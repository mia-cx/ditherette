//! Typed ordinary Processor calls for preparation reuse measurements.

use super::{process::ProcessSettings, quantize::QuantizeSettings};
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, spec::contract::request as spec,
};
use std::io;

/// Warm image-stage fixtures reset and execute this prime before every observed call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SamplePrime {
    SameCall,
    Resize,
    Perturb,
    NoDither,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProcessorSettings {
    Resize {
        output: spec::Output,
    },
    Quantize {
        settings: QuantizeSettings,
    },
    Process {
        settings: ProcessSettings,
    },
    Separable {
        settings: super::fields::SeparableSettings,
    },
}

impl ProcessorSettings {
    pub fn reference_request<'a>(
        &'a self,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<ReferenceRequest<'a>> {
        match self {
            Self::Quantize { settings } => settings.reference_request(source, rgba),
            Self::Process { settings } => settings.reference_request(source, rgba),
            Self::Separable { settings } => settings.reference_request(source, rgba),
            Self::Resize { output } => {
                let request =
                    ReferenceRequest::Processing(spec::Request::Resize(spec::ResizeRequest {
                        version: 1,
                        source: spec::Source {
                            width: source.width,
                            height: source.height,
                            data: rgba,
                        },
                        output: *output,
                    }));
                request.dimensions().map_err(io::Error::other)?;
                Ok(request)
            }
        }
    }

    /// Derive an independent frozen composition for the untimed stage-producing call.
    pub fn prime_request<'a>(
        &'a self,
        prime: SamplePrime,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<ReferenceRequest<'a>> {
        let request = self.reference_request(source, rgba)?;
        let ReferenceRequest::Processing(input) = request else {
            unreachable!("typed processor request")
        };
        let primed = match (prime, input) {
            (SamplePrime::SameCall, input) => input,
            (SamplePrime::Resize, spec::Request::Process(input)) => {
                spec::Request::Resize(spec::ResizeRequest {
                    version: 1,
                    source: input.source,
                    output: input.recipe.output,
                })
            }
            (SamplePrime::Perturb, spec::Request::DitherAndQuantize(input)) => {
                let spec::DitherPolicy::Separable { perturb } = input.dither else {
                    return Err(io::Error::other("prime requires separable settings"));
                };
                spec::Request::Perturb(spec::PerturbRequest {
                    version: 1,
                    source: input.quantize.source,
                    perturb,
                })
            }
            (SamplePrime::NoDither, spec::Request::Quantize(quantize)) => {
                spec::Request::DitherAndQuantize(spec::DitherQuantizeRequest {
                    quantize,
                    dither: spec::DitherPolicy::None {},
                })
            }
            _ => {
                return Err(io::Error::other(
                    "stage prime does not match the measured operation",
                ))
            }
        };
        let result = ReferenceRequest::Processing(primed);
        result.dimensions().map_err(io::Error::other)?;
        Ok(result)
    }

    pub fn reference_subject(&self) -> &'static str {
        match self {
            Self::Resize { .. } => "spec:resize:request:v1",
            Self::Quantize { .. } => "spec:quantize:request:v1",
            Self::Process { .. } => "spec:process:request:v1",
            Self::Separable { .. } => "spec:dither-and-quantize:request:v1",
        }
    }

    pub fn subject(&self) -> &'static str {
        use ditherette_wasm::bench_subjects::{preparation, process};
        match self {
            Self::Resize { .. } => preparation::RESIZE_SUBJECT,
            Self::Quantize { .. } => preparation::QUANTIZE_SUBJECT,
            Self::Process { .. } => process::PROCESS_SUBJECT,
            Self::Separable { .. } => {
                ditherette_wasm::bench_subjects::field_calls::SEPARABLE_SUBJECT
            }
        }
    }
}
