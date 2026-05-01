//! POWL64 Route Proofs.
use insa_instinct::{InstinctByte, KappaByte, SelectedInstinctByte};
use insa_types::{CompletedMask, EdgeId, NodeId};

/// A single step in an admitted process route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(64))]
pub struct RouteCell64 {
    /// Ordinal index
    pub ordinal: u64,
    /// The node id
    pub node: NodeId,
    /// The edge id
    pub edge: EdgeId,
    /// The instinct activated
    pub activation: InstinctByte,
    /// The selected instinct
    pub selected: SelectedInstinctByte,
    /// The collapse attribution
    pub kappa: KappaByte,
    /// Pad to 8-byte alignment
    pub _pad: u8,
    /// The completion state before this step
    pub pre_mask: CompletedMask,
    /// The completion state after this step
    pub post_mask: CompletedMask,
    /// Reserved to exactly 64 bytes total
    _reserved: [u8; 32],
}
