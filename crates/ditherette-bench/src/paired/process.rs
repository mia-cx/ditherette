//! Complete pipeline settings shared by native, public, and frozen-only adapters.

use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::{
    bench_subjects::reference::ReferenceRequest, image::contracts::PaletteEntry,
    spec::contract::request as spec,
};
use std::io;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessSettings {
    pub palette: Vec<PaletteEntry>,
    pub recipe: spec::RecipeV1,
}

impl ProcessSettings {
    /// Validate every stage and borrow the original input before identity construction.
    pub fn reference_request<'a>(
        &'a self,
        source: Dimensions,
        rgba: &'a [u8],
    ) -> io::Result<ReferenceRequest<'a>> {
        let request = ReferenceRequest::Processing(spec::Request::Process(spec::ProcessRequest {
            source: spec::Source {
                width: source.width,
                height: source.height,
                data: rgba,
            },
            palette: &self.palette,
            recipe: self.recipe,
        }));
        request.dimensions().map_err(io::Error::other)?;
        Ok(request)
    }
}
