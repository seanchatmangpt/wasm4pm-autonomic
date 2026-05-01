use insa_instinct::{InstinctByte, KappaByte, PrologByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProofStatus {
    Proved = 0,
    Failed = 1,
    DepthExhausted = 2,
}

impl Default for ProofStatus {
    fn default() -> Self {
        Self::Failed
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProofResult {
    pub status: ProofStatus,
    pub detail: PrologByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
}
