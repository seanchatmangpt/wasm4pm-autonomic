use crate::byte::{InstinctByte, SelectedInstinctByte};

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

#[derive(Debug, Clone, Copy)]
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
}

impl InstinctResolutionLut {
    pub const fn resolve(&self, activation: InstinctByte) -> InstinctResolution {
        let bits = activation.bits() as usize;
        InstinctResolution {
            activation,
            selected: self.selected_lut[bits],
            inhibited: InstinctByte(0), // Simplified for now
            conflict: ConflictStatus::Valid,
            class: self.class_lut[bits],
        }
    }
}
