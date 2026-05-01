use crate::fuse_hearsay::slot::EvidenceSlot;
use insa_types::FieldMask;

/// A bounded blackboard for evidence fusion.
/// Hardcoded to 16 slots to guarantee bounded execution.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blackboard {
    pub slots: [EvidenceSlot; 16],
    pub len: u8,
    pub present: FieldMask,
    pub missing: FieldMask,
    pub conflicted: FieldMask,
    pub stale: FieldMask,
}

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            slots: [EvidenceSlot::default(); 16],
            len: 0,
            present: FieldMask(0),
            missing: FieldMask(0),
            conflicted: FieldMask(0),
            stale: FieldMask(0),
        }
    }
}

impl Blackboard {
    pub fn push(&mut self, slot: EvidenceSlot) -> Result<(), &'static str> {
        if self.len < 16 {
            self.slots[self.len as usize] = slot;
            self.len += 1;
            Ok(())
        } else {
            Err("Blackboard full")
        }
    }
}
