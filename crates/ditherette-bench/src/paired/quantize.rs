//! Complete quantize settings shared by native and actual public-call adapters.

use super::*;
use crate::verification::{input_digest, settings_digest};
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, spec::contract::request as spec,
};
pub use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::contract::request::{AlphaPolicy, MatchPolicy},
};
use std::io;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantizeSettings {
    pub palette: Vec<PaletteEntry>,
    pub alpha: AlphaPolicy,
    pub matching: MatchPolicy,
}

impl QuantizeSettings {
    /// The reference request owns no fixture copy and binds the same palette and alpha semantics.
    pub fn reference_request<'a>(
        &'a self,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<ReferenceRequest<'a>> {
        let matching = match self.matching {
            MatchPolicy::SrgbEuclidean => spec::MatchPolicy::SrgbEuclidean,
            MatchPolicy::LinearRgbEuclidean => spec::MatchPolicy::LinearRgbEuclidean,
            MatchPolicy::OklabEuclidean => spec::MatchPolicy::OklabEuclidean,
            MatchPolicy::CielabEuclidean => spec::MatchPolicy::CielabEuclidean,
            MatchPolicy::YcbcrEuclidean => spec::MatchPolicy::YcbcrEuclidean,
            _ => {
                return Err(io::Error::other(
                    "quantize benchmarks support five ordinary Euclidean spaces only",
                ))
            }
        };
        let alpha = match self.alpha {
            AlphaPolicy::Preserve { threshold } => spec::AlphaPolicy::Preserve { threshold },
            AlphaPolicy::Premultiplied {} => spec::AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb } => spec::AlphaPolicy::Matte { rgb },
        };
        let request =
            ReferenceRequest::Processing(spec::Request::Quantize(spec::QuantizeRequest {
                version: 1,
                source: spec::Source {
                    width: source.width,
                    height: source.height,
                    data: rgba,
                },
                palette: &self.palette,
                alpha,
                matching,
            }));
        request.dimensions().map_err(io::Error::other)?;
        Ok(request)
    }

    pub fn identity(&self, source: Dimensions, rgba: &[u8]) -> io::Result<CaseIdentity> {
        let request = self.reference_request(source, rgba)?;
        Ok(CaseIdentity {
            semantics: request.semantics(),
            input: input_digest(source, rgba),
            settings: settings_digest(&request).map_err(io::Error::other)?,
            output: source,
        })
    }
}
