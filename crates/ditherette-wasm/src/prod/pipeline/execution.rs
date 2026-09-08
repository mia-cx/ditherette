//! Private complete-call candidates. Defaults stay scalar until measured policy is accepted.

use crate::prod::tiling::WorkerBudget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBandPolicy {
    pub height: u32,
    pub workers: WorkerBudget,
    pub active_workers: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionPolicy {
    pub resize: Option<RowBandPolicy>,
    pub indexed: Option<RowBandPolicy>,
    pub mixing: Option<RowBandPolicy>,
}
