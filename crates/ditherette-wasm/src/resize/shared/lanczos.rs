use crate::error::ProcessingError;

pub(crate) fn validate_window_size(window_size: f64) -> Result<(), ProcessingError> {
    if window_size.is_finite() && window_size > 0.0 {
        Ok(())
    } else {
        Err(ProcessingError::InvalidParameter {
            name: "window_size",
            reason: "must be finite and greater than zero",
        })
    }
}
