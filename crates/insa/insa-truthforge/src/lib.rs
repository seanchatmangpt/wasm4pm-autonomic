#![doc = include_str!("../README.md")]
//! Comprehensive verification harness for INSA.
//!
//! This crate acts as the central gatekeeper, containing property tests,
//! benchmarks, and compile-fail assertions to guarantee the layout and
//! semantic invariants of the INSA ecosystem.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod admission;
pub mod gates;
