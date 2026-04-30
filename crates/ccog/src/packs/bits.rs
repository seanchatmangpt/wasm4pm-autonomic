//! Bit namespace allocation for field packs (Phase 12).
//!
//! Each pack owns a non-overlapping range of posture/context bits. The
//! `const_assert!`s in this module and at each pack site enforce that ranges
//! never overlap and never straddle the 64-bit canonical boundary.
//!
//! | Range | Owner |
//! |---|---|
//! | 0–15  | core (frozen) |
//! | 16–31 | Lifestyle / OT |
//! | 32–47 | Edge / Home |
//! | 48–55 | Enterprise |
//! | 56–63 | Dev / Agent Governance |
//!
//! Adding a new pack must extend the table here and add a fresh non-overlapping
//! `const_assert!` to its module. Bits 0–15 are reserved for canonical core
//! posture/context predicates and MUST NOT be reissued by any pack.

use core::ops::Range;

/// Frozen core range — reserved for canonical posture/context bits.
pub const CORE_RANGE: Range<u32> = 0..16;

/// Lifestyle / OT range.
pub const LIFESTYLE_RANGE: Range<u32> = 16..32;

/// Edge / Home range.
pub const EDGE_RANGE: Range<u32> = 32..48;

/// Enterprise range.
pub const ENTERPRISE_RANGE: Range<u32> = 48..56;

/// Dev / Agent Governance range.
pub const DEV_RANGE: Range<u32> = 56..64;

/// Compile-time const_assert macro (re-implemented locally to avoid pulling a
/// crate dep — Phase 12 keeps the pack subsystem dep-free).
#[macro_export]
macro_rules! ccog_const_assert {
    ($cond:expr $(,)?) => {
        const _: [(); 0 - !{
            const ASSERT: bool = $cond;
            ASSERT
        } as usize] = [];
    };
}

// --- Cross-range non-overlap proofs (compile-time) ---

ccog_const_assert!(CORE_RANGE.end <= LIFESTYLE_RANGE.start);
ccog_const_assert!(LIFESTYLE_RANGE.end <= EDGE_RANGE.start);
ccog_const_assert!(EDGE_RANGE.end <= ENTERPRISE_RANGE.start);
ccog_const_assert!(ENTERPRISE_RANGE.end <= DEV_RANGE.start);
ccog_const_assert!(DEV_RANGE.end <= 64);

/// True iff `bit` falls within `range`.
#[must_use]
pub const fn in_range(bit: u32, range: Range<u32>) -> bool {
    bit >= range.start && bit < range.end
}

// =============================================================================
// Phase 7 — Lifestyle K-tier bit allocations.
// K-tier masks live OUTSIDE the existing posture/context u64 surface.
// Each K-tier is its own u64; field bit ranges are non-overlapping by
// const_assert!. Phase 7.1 ships Routine + Capacity + Safety in K1,
// Meaning in K2, and Evidence in K3.
// =============================================================================

/// K1 — Routine / Capacity / Regulation / Safety.
pub mod k1 {
    use core::ops::Range;
    /// Routine cadence bits (e.g. ROUTINE_DUE, ROUTINE_MISSED).
    pub const ROUTINE_RANGE: Range<u32> = 0..16;
    /// Capacity / energy bits (e.g. FATIGUE_HIGH, LOW_EXEC_CAPACITY).
    pub const CAPACITY_RANGE: Range<u32> = 16..32;
    /// Regulation / overstim bits (Phase 7.2 — reserved).
    pub const REGULATION_RANGE: Range<u32> = 32..48;
    /// Safety / risk bits (e.g. DRIVING_RISK, MEDICATION_OVERDUE).
    pub const SAFETY_RANGE: Range<u32> = 48..64;
}
crate::ccog_const_assert!(k1::ROUTINE_RANGE.end <= k1::CAPACITY_RANGE.start);
crate::ccog_const_assert!(k1::CAPACITY_RANGE.end <= k1::REGULATION_RANGE.start);
crate::ccog_const_assert!(k1::REGULATION_RANGE.end <= k1::SAFETY_RANGE.start);
crate::ccog_const_assert!(k1::SAFETY_RANGE.end <= 64);

/// K2 — Meaning / Social / Recovery / Identity.
pub mod k2 {
    use core::ops::Range;
    /// Meaningful occupation bits.
    pub const MEANING_RANGE: Range<u32> = 0..16;
    /// Social / relational bits (Phase 7.2 — reserved).
    pub const SOCIAL_RANGE: Range<u32> = 16..32;
    /// Recovery / sobriety bits (Phase 7.2 — reserved).
    pub const RECOVERY_RANGE: Range<u32> = 32..48;
    /// Identity bits (Phase 7.2 — reserved).
    pub const IDENTITY_RANGE: Range<u32> = 48..64;
}
crate::ccog_const_assert!(k2::MEANING_RANGE.end <= k2::SOCIAL_RANGE.start);
crate::ccog_const_assert!(k2::SOCIAL_RANGE.end <= k2::RECOVERY_RANGE.start);
crate::ccog_const_assert!(k2::RECOVERY_RANGE.end <= k2::IDENTITY_RANGE.start);
crate::ccog_const_assert!(k2::IDENTITY_RANGE.end <= 64);

/// K3 — Environment / Object / Transition / Evidence / Rhythm.
pub mod k3 {
    use core::ops::Range;
    /// Environment / place bits (Phase 7.2 — reserved).
    pub const ENVIRONMENT_RANGE: Range<u32> = 0..13;
    /// Object / tool bits (Phase 7.2 — reserved).
    pub const OBJECT_RANGE: Range<u32> = 13..26;
    /// Transition friction bits (Phase 7.2 — reserved).
    pub const TRANSITION_RANGE: Range<u32> = 26..38;
    /// Evidence-gap bits (e.g. MEAL_EVIDENCE_MISSING).
    pub const EVIDENCE_RANGE: Range<u32> = 38..51;
    /// Temporal rhythm bits (Phase 7.2 — reserved).
    pub const RHYTHM_RANGE: Range<u32> = 51..64;
}
crate::ccog_const_assert!(k3::ENVIRONMENT_RANGE.end <= k3::OBJECT_RANGE.start);
crate::ccog_const_assert!(k3::OBJECT_RANGE.end <= k3::TRANSITION_RANGE.start);
crate::ccog_const_assert!(k3::TRANSITION_RANGE.end <= k3::EVIDENCE_RANGE.start);
crate::ccog_const_assert!(k3::EVIDENCE_RANGE.end <= k3::RHYTHM_RANGE.start);
crate::ccog_const_assert!(k3::RHYTHM_RANGE.end <= 64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_pairwise_disjoint() {
        let ranges = [
            ("core", CORE_RANGE),
            ("lifestyle", LIFESTYLE_RANGE),
            ("edge", EDGE_RANGE),
            ("enterprise", ENTERPRISE_RANGE),
            ("dev", DEV_RANGE),
        ];
        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                let (na, ra) = (ranges[i].0, ranges[i].1.clone());
                let (nb, rb) = (ranges[j].0, ranges[j].1.clone());
                assert!(
                    ra.end <= rb.start || rb.end <= ra.start,
                    "{na} and {nb} overlap"
                );
            }
        }
    }

    #[test]
    fn dev_range_terminates_at_canonical_boundary() {
        assert_eq!(DEV_RANGE.end, 64);
    }

    #[test]
    fn in_range_boundary_is_half_open() {
        assert!(in_range(16, LIFESTYLE_RANGE));
        assert!(!in_range(32, LIFESTYLE_RANGE));
        assert!(in_range(31, LIFESTYLE_RANGE));
    }
}
