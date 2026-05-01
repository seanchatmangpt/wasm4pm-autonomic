use insa_instinct::{DendralByte, InstinctByte, KappaByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DendralStatus {
    Unique = 0,
    Ambiguous = 1,
    Failed = 2,
}

impl Default for DendralStatus {
    fn default() -> Self {
        Self::Failed
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DendralResult {
    pub status: DendralStatus,
    pub detail: DendralByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
}
