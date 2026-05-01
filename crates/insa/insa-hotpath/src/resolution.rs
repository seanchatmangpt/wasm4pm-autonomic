//! Instinct Resolution structure.
use insa_instinct::{InstinctByte, SelectedInstinctByte};

/// InstinctResolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstinctResolution {
    /// activation
    pub activation: InstinctByte,
    /// selected
    pub selected: SelectedInstinctByte,
    /// inhibited
    pub inhibited: u8,
    /// conflict
    pub conflict: bool,
}
