//! Borrowed callback handles and caught scalar/void imports for synchronous progress.

use crate::prod::{
    contract::{
        error::ErrorCode,
        failure::{ErrorPath, Failure},
        lifecycle::{Progress, Stage},
    },
    pipeline::progress::Callback,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/wasm/progress_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = progressEnabled)]
    fn progress_enabled(sink: &JsValue) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = progressClock)]
    fn progress_clock() -> Result<f64, JsValue>;
    #[wasm_bindgen(catch, js_name = reportProgress)]
    fn report_progress(
        sink: &JsValue,
        stage: u32,
        completed: f64,
        total: f64,
    ) -> Result<(), JsValue>;
}

pub(super) struct JsProgress<'a> {
    sink: &'a JsValue,
}

impl<'a> JsProgress<'a> {
    pub(super) fn new(sink: &'a JsValue) -> Result<Option<Self>, Failure> {
        progress_enabled(sink)
            .map(|enabled| enabled.then_some(Self { sink }))
            .map_err(|_| Failure::new(ErrorCode::InvalidSettings, ErrorPath::OnProgress))
    }
}

impl Callback for JsProgress<'_> {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        progress_clock()
            .map(|time| time as u64)
            .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))
    }
    fn report(&mut self, progress: Progress) -> Result<(), ()> {
        let stage = match progress.stage {
            Stage::Prepare => 0,
            Stage::Resize => 1,
            Stage::Alpha => 2,
            Stage::Color => 3,
            Stage::Perturb => 4,
            Stage::Quantize => 5,
            Stage::DitherAndQuantize => 6,
            Stage::Complete => 7,
        };
        report_progress(
            self.sink,
            stage,
            progress.completed.map_or(f64::NAN, |value| value as f64),
            progress.total.map_or(f64::NAN, |value| value as f64),
        )
        .map_err(|_| ())
    }
}
