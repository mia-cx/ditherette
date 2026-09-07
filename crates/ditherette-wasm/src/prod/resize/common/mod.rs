//! Shared production resize primitives.
//!
//! These helpers are duplicated from the spec where needed so production code
//! does not cross the oracle boundary.

pub mod alignment;
pub mod coordinates;
pub mod rgba8;
pub mod sample;
