//! Lookup Tables (LUTs) for O(1) resolution.

/// Exhaustive resolution tables.
pub struct InstinctResolutionLut {
    /// inhibition
    pub inhibition: [u8; 256],
    /// conflict
    pub conflict: [u8; 256],
    /// selected
    pub selected: [u8; 256],
    /// resolution_class
    pub resolution_class: [u8; 256],
}
