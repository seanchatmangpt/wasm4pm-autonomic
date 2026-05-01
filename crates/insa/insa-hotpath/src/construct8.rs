//! CONSTRUCT8 bounded delta generation.
//!
//! Prevents runaway state mutations by forcing external output into bounded
//! graph deltas. The size of the delta must never exceed 8 operations.

use insa_types::FieldMask;

/// The type of modification to apply to the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Construct8OpKind {
    /// No operation / empty slot.
    #[default]
    None = 0,
    /// Set the field bit to present.
    Set = 1,
    /// Clear the field bit.
    Clear = 2,
}

/// A single bounded mutation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Construct8Op {
    /// The operation kind.
    pub kind: Construct8OpKind,
    /// The bit index in the field mask (0-63).
    pub bit_index: u8,
}

/// A bounded state mutation allowed to re-enter the field.
///
/// Strictly bounded to 8 operations to enforce the CONSTRUCT8 law.
/// Allocates zero octets on the heap.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Construct8Delta {
    /// Number of valid operations in this delta.
    pub len: u8,
    /// Up to 8 bounded mutations.
    pub ops: [Construct8Op; 8],
}

impl Construct8Delta {
    /// Creates a new empty bounded delta.
    pub const fn new() -> Self {
        Self {
            len: 0,
            ops: [Construct8Op {
                kind: Construct8OpKind::None,
                bit_index: 0,
            }; 8],
        }
    }

    /// Attempts to push a new mutation operation into the bounded delta.
    pub const fn push(mut self, op: Construct8Op) -> Result<Self, &'static str> {
        if self.len < 8 {
            self.ops[self.len as usize] = op;
            self.len += 1;
            Ok(self)
        } else {
            Err("CONSTRUCT8 violation: delta exceeded 8 mutations")
        }
    }

    /// Applies the delta to a given field mask.
    pub const fn apply(&self, mut current: FieldMask) -> FieldMask {
        let mut i = 0;
        while i < self.len as usize {
            let op = &self.ops[i];
            let mask = 1 << op.bit_index;
            match op.kind {
                Construct8OpKind::Set => {
                    current.0 |= mask;
                }
                Construct8OpKind::Clear => {
                    current.0 &= !mask;
                }
                Construct8OpKind::None => {}
            }
            i += 1;
        }
        current
    }
}
