//! Typed diffusion settings bind every feedback and placement control to case identity.

use super::{quantize::QuantizeSettings, *};
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, spec::contract::request as spec,
};
use std::io;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffusionSettings {
    pub quantize: QuantizeSettings,
    pub kernel: spec::Diffusion,
    pub feedback: spec::DiffusionFeedback,
    pub strength: f32,
    pub serpentine: bool,
    pub placement: spec::Placement,
}

impl DiffusionSettings {
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
                dither: spec::DitherPolicy::Diffusion {
                    kernel: self.kernel,
                    feedback: self.feedback,
                    strength: self.strength,
                    serpentine: self.serpentine,
                    placement: self.placement,
                },
            },
        ));
        request.dimensions().map_err(io::Error::other)?;
        Ok(request)
    }
}
