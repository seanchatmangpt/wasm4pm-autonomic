//! Semantic bitmask definitions.

/// Represents the presence of evidence fields.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FieldMask(pub u64);

/// Represents the completion of cognitive processes/nodes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct CompletedMask(pub u64);

/// Identifies a specific bit within a 64-bit field mask.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldBit(u8);

impl FieldBit {
    /// Creates a checked FieldBit.
    pub const fn new_checked(value: u8) -> Result<Self, &'static str> {
        if value < 64 {
            Ok(Self(value))
        } else {
            Err("FieldBit must be in range [0, 63]")
        }
    }
}
