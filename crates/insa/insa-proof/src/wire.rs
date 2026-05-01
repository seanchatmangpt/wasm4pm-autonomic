//! Canonical file encoding contracts.

/// Magic bytes for POWL64 V1 format.
pub const POWL64_V1_MAGIC: [u8; 8] = *b"POWL64\x00\x01";

/// 256-byte explicitly padded header for canonical wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct WirePowl64HeaderV1 {
    /// Magic bytes identifier
    pub magic: [u8; 8],
    /// Format version
    pub version: u16,
    /// Explicit tail padding to ensure exact 256 bytes total (256 - 10 = 246)
    pub reserved: [u8; 246],
}
