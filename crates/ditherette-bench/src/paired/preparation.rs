//! Typed ordinary Processor calls for preparation reuse measurements.

use super::{process::ProcessSettings, quantize::QuantizeSettings};
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, spec::contract::request as spec,
};
use std::io;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "method", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProcessorSettings {
    Resize { output: spec::Output },
    Quantize { settings: QuantizeSettings },
    Process { settings: ProcessSettings },
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

    pub fn reference_subject(&self) -> &'static str {
        match self {
            Self::Resize { .. } => "spec:resize:request:v1",
            Self::Quantize { .. } => "spec:quantize:request:v1",
            Self::Process { .. } => "spec:process:request:v1",
        }
    }

    pub fn subject(&self) -> &'static str {
        use ditherette_wasm::bench_subjects::{preparation, process};
        match self {
            Self::Resize { .. } => preparation::RESIZE_SUBJECT,
            Self::Quantize { .. } => preparation::QUANTIZE_SUBJECT,
            Self::Process { .. } => process::PROCESS_SUBJECT,
        }
    }
}
