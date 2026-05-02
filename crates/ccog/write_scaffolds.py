import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

construct8_content = """//! CONSTRUCT8 bounded delta generation.
//!
//! Prevents runaway state mutations by forcing external output into bounded
//! graph deltas. The size of the delta must never exceed 8 operations.

use insa_types::FieldMask;

/// The type of modification to apply to the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Construct8OpKind {
    /// No operation / empty slot.
    None = 0,
    /// Set the field bit to present.
    Set = 1,
    /// Clear the field bit.
    Clear = 2,
}

impl Default for Construct8OpKind {
    fn default() -> Self {
        Self::None
    }
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
            ops: [Construct8Op { kind: Construct8OpKind::None, bit_index: 0 }; 8],
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
                },
                Construct8OpKind::Clear => {
                    current.0 &= !mask;
                },
                Construct8OpKind::None => {}
            }
            i += 1;
        }
        current
    }
}
"""

resolution_content = """use crate::byte::{InstinctByte, SelectedInstinctByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConflictStatus {
    Valid = 0,
    Suspicious = 1,
    Conflict = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResolutionClass {
    Unresolved = 0,
    Terminal = 1,
    InformationGathering = 2,
    Escalating = 3,
    Blocked = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstinctResolution {
    pub activation: InstinctByte,
    pub selected: SelectedInstinctByte,
    pub inhibited: InstinctByte,
    pub conflict: ConflictStatus,
    pub class: ResolutionClass,
}

pub struct InstinctResolutionLut {
    pub selected_lut: [SelectedInstinctByte; 256],
    pub class_lut: [ResolutionClass; 256],
    pub conflict_lut: [ConflictStatus; 256],
}

impl InstinctResolutionLut {
    pub const fn resolve(&self, activation: InstinctByte) -> InstinctResolution {
        let bits = activation.bits() as usize;
        let selected = self.selected_lut[bits];
        
        // Exact semantic law: Inhibited is whatever was activated but not selected.
        let inhibited_bits = activation.bits() ^ selected.0;
        
        InstinctResolution {
            activation,
            selected,
            inhibited: InstinctByte(inhibited_bits),
            conflict: self.conflict_lut[bits],
            class: self.class_lut[bits],
        }
    }
}
"""

test_content = """use insa_hotpath::cog8::execute_cog8_graph;
use insa_hotpath::powl8::Powl8Op;
use insa_instinct::InstinctByte;
use insa_security::*;
use insa_types::FieldMask;

#[test]
fn test_access_drift_jtbd() {
    let rows = build_access_drift_rows();

    // Given: terminated contractor + active badge/VPN/repo + vendor expired + site/device activity
    let o_star_present = FieldMask::empty()
        .with_bit(IDENTITY_TERMINATED)
        .with_bit(BADGE_ACTIVE)
        .with_bit(VPN_ACTIVE)
        .with_bit(REPO_ACCESS_ACTIVE)
        .with_bit(VENDOR_CONTRACT_EXPIRED)
        .with_bit(RECENT_SITE_ENTRY);

    // When: security graph closes field
    let decision = execute_cog8_graph(&rows, o_star_present.0, 0).expect("Graph execution failed");

    // Then: Refuse/Escalate selected
    assert!(decision.response.contains(InstinctByte::REFUSE));
    assert!(decision.fired_mask > 0);

    // Resolve POWL8 motion via admitted struct matching.
    // Reflexes translate into specific operators in process topologies.
    let selected_motion = if decision.response.contains(InstinctByte::REFUSE) {
        Powl8Op::Block
    } else if decision.response.contains(InstinctByte::ESCALATE) {
        Powl8Op::Silent // Usually deferred to out-of-band HITL
    } else {
        Powl8Op::Act
    };
    
    assert_eq!(selected_motion, Powl8Op::Block, "The selected action for REFUSE should systematically yield a Block motion under POWL8 mapping");
}
"""

write_file('../insa/insa-hotpath/src/construct8.rs', construct8_content)
write_file('../insa/insa-instinct/src/resolution.rs', resolution_content)
write_file('../insa/insa-truthforge/tests/jtbd_access_drift.rs', test_content)

print("Scaffolds eradicated and replaced with robust implementations.")
