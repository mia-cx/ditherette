//! Synchronous operation execution through the readable lifecycle state model.
//!
//! Callbacks run without an active RefCell borrow, so recursive calls reach the
//! model's structured rejection instead of panicking on a Rust borrow conflict.

use std::cell::RefCell;

use super::{execute_with_progress, ProcessedImage};
use crate::spec::contract::{
    error::DitheretteError,
    lifecycle::{InstanceModel, Progress, Stage},
    request::Request,
};

/// A callback error models a thrown host callback, not a recoverable kernel result.
pub type ProgressCallback<'a> = dyn FnMut(Progress) -> Result<(), ()> + 'a;

/// Independent reference instance. Production adds bounded allocation and caching separately.
#[derive(Debug, Default)]
pub struct Processor {
    model: RefCell<InstanceModel>,
}

impl Processor {
    /// Executes a complete operation. Only successful completion returns owned output.
    /// The supplied monotonic clock controls the same 50 ms throttle as the host adapter.
    pub fn execute(
        &self,
        request: Request<'_>,
        mut on_progress: Option<&mut ProgressCallback<'_>>,
        mut now_ms: impl FnMut() -> u64,
    ) -> Result<ProcessedImage, DitheretteError> {
        self.model.borrow_mut().begin(on_progress.is_some())?;
        let result = execute_with_progress(request, &mut |progress| {
            self.report(progress, &mut on_progress, now_ms())
        });
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                // A callback failure already restores Ready. Ordinary failures still need fail().
                if error.code != crate::spec::contract::error::ErrorCode::Callback {
                    self.model.borrow_mut().fail()?;
                }
                return Err(error);
            }
        };
        self.model.borrow_mut().output_ready()?;
        self.report(
            Progress {
                stage: Stage::Complete,
                completed: Some(output.pixel_count()),
                total: Some(output.pixel_count()),
            },
            &mut on_progress,
            now_ms(),
        )?;
        self.model.borrow_mut().finish()?;
        Ok(output)
    }

    /// Idempotently disposes this instance. Disposal inside a callback is rejected.
    pub fn dispose(&self) -> Result<(), DitheretteError> {
        self.model.borrow_mut().dispose()
    }

    fn report(
        &self,
        progress: Progress,
        callback: &mut Option<&mut ProgressCallback<'_>>,
        now_ms: u64,
    ) -> Result<(), DitheretteError> {
        if !self.model.borrow_mut().report(progress, now_ms)? {
            return Ok(());
        }
        let callback = callback.as_mut().expect("enabled callback is present");
        if callback(progress).is_err() {
            return Err(self.model.borrow_mut().callback_failed());
        }
        self.model.borrow_mut().callback_succeeded()
    }
}
