//! Reference Law Path for INSA execution.
//!
//! This crate contains the semantic oracle for COG8, POWL8, CONSTRUCT8, and
//! Instinct resolution.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

pub mod cog8;
pub mod construct8;
pub mod lut;
pub mod powl8;
pub mod resolution;

pub use crate::cog8::*;
pub use crate::construct8::*;
pub use crate::lut::*;
pub use crate::powl8::*;
pub use crate::resolution::*;
