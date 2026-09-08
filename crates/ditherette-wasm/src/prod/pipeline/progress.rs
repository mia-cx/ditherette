//! Per-call progress gating using the copied lifecycle model and borrowed callbacks.

use crate::prod::contract::{
    error::ErrorCode,
    failure::{ErrorPath, Failure},
    lifecycle::{InstanceModel, Progress, Stage},
};

/// The adapter catches external calls. Disabled progress never reads the clock.
pub trait Callback {
    fn now_ms(&mut self) -> Result<u64, Failure>;
    fn report(&mut self, progress: Progress) -> Result<(), ()>;
}

/// Owned inline control only; callbacks and event storage remain with the boundary.
pub(crate) struct Control {
    model: InstanceModel,
}

impl Control {
    pub(crate) fn new(enabled: bool) -> Self {
        let mut model = InstanceModel::default();
        model.begin(enabled).expect("new call control is idle");
        Self { model }
    }

    pub(crate) fn report(
        &mut self,
        callback: Option<&mut dyn Callback>,
        stage: Stage,
        completed: u64,
        total: u64,
    ) -> Result<(), Failure> {
        let Some(callback) = callback else {
            return Ok(());
        };
        let event = Progress {
            stage,
            completed: Some(completed),
            total: Some(total),
        };
        if !self
            .model
            .report(event, callback.now_ms()?)
            .map_err(|_| control_failure())?
        {
            return Ok(());
        }
        // A failed call discards this model. Avoid its allocating diagnostic constructor.
        callback
            .report(event)
            .map_err(|()| Failure::new(ErrorCode::Callback, ErrorPath::OnProgress))?;
        self.model
            .callback_succeeded()
            .map_err(|_| control_failure())
    }

    /// The caller has already constructed its durable output, but has not committed cache entries.
    pub(crate) fn complete(&mut self, callback: Option<&mut dyn Callback>) -> Result<(), Failure> {
        self.model.output_ready().map_err(|_| control_failure())?;
        self.report(callback, Stage::Complete, 1, 1)?;
        self.model.finish().map_err(|_| control_failure())
    }

    /// Drops an already-built output if completion throws; the caller still owns publication.
    pub(crate) fn finish<T>(
        &mut self,
        result: Result<T, Failure>,
        callback: Option<&mut dyn Callback>,
    ) -> Result<T, Failure> {
        let output = result?;
        self.complete(callback)?;
        Ok(output)
    }
}

fn control_failure() -> Failure {
    Failure::new(ErrorCode::Runtime, ErrorPath::Control)
}

#[cfg(test)]
mod kernel_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Clock {
        now: u64,
        events: Vec<Progress>,
        fail: bool,
    }

    impl Callback for Clock {
        fn now_ms(&mut self) -> Result<u64, Failure> {
            Ok(self.now)
        }
        fn report(&mut self, progress: Progress) -> Result<(), ()> {
            self.events.push(progress);
            if self.fail {
                Err(())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn thrown_completion_fails_without_success_permission() {
        let mut clock = Clock {
            fail: true,
            ..Clock::default()
        };
        let error = Control::new(true).complete(Some(&mut clock)).unwrap_err();
        assert_eq!(
            error,
            Failure::new(ErrorCode::Callback, ErrorPath::OnProgress)
        );
        Control::new(false).complete(None).unwrap();
        clock.fail = false;
        Control::new(true).complete(Some(&mut clock)).unwrap();
    }
}
