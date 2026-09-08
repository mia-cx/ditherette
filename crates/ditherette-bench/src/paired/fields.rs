//! Complete field settings shared by native and actual public adapters.

use super::{quantize::QuantizeSettings, *};
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, spec::contract::request as spec,
};
pub use spec::{BayerSize, Field, PerturbPolicy, Placement, WorkingSpace};
use std::io;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeparableSettings {
    pub quantize: QuantizeSettings,
    pub perturb: PerturbPolicy,
}

pub fn perturb_request<'a>(
    settings: PerturbPolicy,
    source: Dimensions,
    rgba: &'a [u8],
) -> io::Result<ReferenceRequest<'a>> {
    let request = ReferenceRequest::Processing(spec::Request::Perturb(spec::PerturbRequest {
        version: 1,
        source: spec::Source {
            width: source.width,
            height: source.height,
            data: rgba,
        },
        perturb: settings,
    }));
    request.dimensions().map_err(io::Error::other)?;
    Ok(request)
}

impl SeparableSettings {
    pub fn reference_request<'a>(
        &'a self,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<ReferenceRequest<'a>> {
        let ReferenceRequest::Processing(spec::Request::Quantize(quantize)) =
            self.quantize.reference_request(source, rgba)?
        else {
            unreachable!("typed quantize settings")
        };
        let request = ReferenceRequest::Processing(spec::Request::DitherAndQuantize(
            spec::DitherQuantizeRequest {
                quantize,
                dither: spec::DitherPolicy::Separable {
                    perturb: self.perturb,
                },
            },
        ));
        request.dimensions().map_err(io::Error::other)?;
        Ok(request)
    }
}
