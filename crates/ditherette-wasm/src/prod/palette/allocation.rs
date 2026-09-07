//! Fallible capacity reservation for prepared palette and quantizer ownership.

use crate::prod::contract::error::ErrorCode;
use std::mem::size_of;

/// Allocation-free failure for the future public adapter to translate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparationError {
    pub code: ErrorCode,
    pub path: &'static str,
}

impl PreparationError {
    pub(crate) const fn memory() -> Self {
        Self {
            code: ErrorCode::MemoryLimit,
            path: "memoryLimitBytes",
        }
    }
    pub(crate) const fn allocation() -> Self {
        Self {
            code: ErrorCode::WasmMemoryUnavailable,
            path: "wasm",
        }
    }
}

pub(crate) struct Budget {
    limit: u64,
    pub used: u64,
}

impl Budget {
    pub fn new(limit: u64, fixed: u64) -> Result<Self, PreparationError> {
        if fixed > limit {
            return Err(PreparationError::memory());
        }
        Ok(Self { limit, used: fixed })
    }

    pub fn reserve<T>(
        &mut self,
        output: &mut Vec<T>,
        count: usize,
    ) -> Result<(), PreparationError> {
        debug_assert!(output.is_empty() && output.capacity() == 0);
        let bytes = (count as u64)
            .checked_mul(size_of::<T>() as u64)
            .ok_or_else(PreparationError::memory)?;
        self.check(bytes)?;
        output
            .try_reserve_exact(count)
            .map_err(|_| PreparationError::allocation())?;
        self.used += output.capacity() as u64 * size_of::<T>() as u64;
        self.check(0)
    }

    pub fn string(&mut self, value: &str) -> Result<String, PreparationError> {
        self.check(value.len() as u64)?;
        let mut output = String::new();
        output
            .try_reserve_exact(value.len())
            .map_err(|_| PreparationError::allocation())?;
        self.used += output.capacity() as u64;
        self.check(0)?;
        output.push_str(value);
        Ok(output)
    }

    fn check(&self, additional: u64) -> Result<(), PreparationError> {
        if self
            .used
            .checked_add(additional)
            .map_or(true, |bytes| bytes > self.limit)
        {
            return Err(PreparationError::memory());
        }
        Ok(())
    }
}
