use insa_instinct::{InstinctByte, KappaByte, MycinByte};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MycinStatus {
    Fired = 0,
    Conflict = 1,
    NoMatch = 2,
}

impl Default for MycinStatus {
    fn default() -> Self {
        Self::NoMatch
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MycinResult {
    pub status: MycinStatus,
    pub detail: MycinByte,
    pub kappa: KappaByte,
    pub emits: InstinctByte,
}
