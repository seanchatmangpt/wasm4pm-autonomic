//! CONSTRUCT8 bounded delta generation.

/// A bounded state mutation allowed to re-enter the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Construct8Delta {
    /// The fields mutated
    pub fields: Vec<u64>,
}
