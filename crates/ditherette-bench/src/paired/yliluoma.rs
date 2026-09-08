//! Complete scalar Yliluoma settings bind palette, alpha, metric, matrix, and placement.

use super::quantize::QuantizeSettings;
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, spec::contract::request as spec,
};
use std::io;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YliluomaSettings {
    pub quantize: QuantizeSettings,
    pub size: spec::BayerSize,
    pub placement: spec::Placement,
}

impl YliluomaSettings {
    /// Validate and borrow the frozen request before identities or samples are created.
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
                dither: spec::DitherPolicy::Yliluoma {
                    size: self.size,
                    placement: self.placement,
                },
            },
        ));
        request.dimensions().map_err(io::Error::other)?;
        Ok(request)
    }
}
