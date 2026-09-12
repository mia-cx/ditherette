//! Common resize spec semantics shared by scalar spec implementations.
//!
//! Modules here define resize-wide semantic contracts, such as coordinate
//! alignment and sample conversion. They stay in the spec tree and remain
//! readable oracle code, not production helpers.

pub mod alignment;
pub mod coordinates;
pub mod sample;
