//! Capacity-accounted fallible reservations for private resize plans and scratch.

use std::mem::size_of;

use crate::prod::contract::{
    error::ErrorCode,
    failure::{ErrorPath, Failure},
};

/// One preparation's allocation ledger. Records live vector capacity, not element count.
pub struct CapacityBudget {
    limit: u64,
    used: u64,
}

impl CapacityBudget {
    pub const fn new(limit: u64) -> Self {
        Self { limit, used: 0 }
    }

    pub const fn used(&self) -> u64 {
        self.used
    }

    /// Reserve an empty vector. Filling at most `capacity` elements cannot allocate again.
    pub fn vector<T>(&mut self, capacity: usize) -> Result<Vec<T>, Failure> {
        let requested = (capacity as u64).checked_mul(size_of::<T>() as u64);
        if requested
            .and_then(|bytes| self.used.checked_add(bytes))
            .is_none_or(|bytes| bytes > self.limit)
        {
            return Err(Failure::new(
                ErrorCode::MemoryLimit,
                ErrorPath::MemoryLimitBytes,
            ));
        }
        let mut vector = Vec::new();
        vector
            .try_reserve_exact(capacity)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))?;
        let actual = vector.capacity() as u64 * size_of::<T>() as u64;
        self.used += actual;
        if self.used > self.limit {
            return Err(Failure::new(
                ErrorCode::MemoryLimit,
                ErrorPath::MemoryLimitBytes,
            ));
        }
        Ok(vector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_nested_headers_and_element_capacity() {
        let required = size_of::<Vec<f64>>() as u64 + 3 * 8;
        let mut budget = CapacityBudget::new(required);
        let mut rows = budget.vector::<Vec<f64>>(1).unwrap();
        let mut row = budget.vector::<f64>(3).unwrap();
        row.extend([1.0, 2.0, 3.0]);
        rows.push(row);
        assert_eq!(budget.used(), required);
        assert_eq!(
            budget.vector::<u8>(1).unwrap_err().code,
            ErrorCode::MemoryLimit
        );
        assert_eq!(rows[0], [1.0, 2.0, 3.0]);
    }

    #[test]
    fn rejects_oversized_reservations_before_allocation() {
        let mut budget = CapacityBudget::new(7);
        assert_eq!(
            budget.vector::<f64>(1).unwrap_err().code,
            ErrorCode::MemoryLimit
        );
        assert_eq!(budget.used(), 0);
    }
}
