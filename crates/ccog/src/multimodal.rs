//! Multimodal posture and context bundles (Phase 6).
//!
//! `PostureBundle` captures interpreter posture as a 64-bit mask over
//! [`PostureBit`] positions — audio cadence (delivery vs partner), gaze
//! orientation (entry vs interior), body tension (calm/alert/engaged), and
//! settling. `ContextBundle` captures local cognition surface as three
//! parallel masks — expectation, risk, and affordance — over [`ContextBit`]
//! positions. The bundles are immutable, `Copy`-able, and consumed by the
//! [`crate::instinct::select_instinct_v0`] decision lattice.

/// Bit positions for canonical posture predicates.
#[allow(non_snake_case)]
pub mod PostureBit {
    /// Subject is calm — no current alert.
    pub const CALM: u32 = 0;
    /// Subject has detected a single signal.
    pub const ALERT: u32 = 1;
    /// Subject has multiple corroborating signals.
    pub const ENGAGED: u32 = 2;
    /// Subject has resolved its stance, returned to baseline.
    pub const SETTLED: u32 = 3;
    /// Subject orientation toward an entry/door.
    pub const ORIENTED_TO_ENTRY: u32 = 4;
    /// Subject orientation toward an internal source.
    pub const ORIENTED_INTERIOR: u32 = 5;
    /// Cadence indicates a known package/delivery class.
    pub const CADENCE_DELIVERY: u32 = 6;
    /// Cadence indicates a known wife/partner arrival class.
    pub const CADENCE_PARTNER: u32 = 7;
}

/// Bit positions for canonical context predicates.
#[allow(non_snake_case)]
pub mod ContextBit {
    /// A package is currently expected.
    pub const PACKAGE_EXPECTED: u32 = 0;
    /// Partner arrival is currently expected.
    pub const PARTNER_DUE: u32 = 1;
    /// Maintenance is currently scheduled.
    pub const MAINTENANCE_SCHEDULED: u32 = 2;
    /// Theft risk is currently elevated.
    pub const THEFT_RISK: u32 = 3;
    /// Safety risk is currently elevated.
    pub const SAFETY_RISK: u32 = 4;
    /// Retrieval is currently low-cost / available.
    pub const CAN_RETRIEVE_NOW: u32 = 5;
    /// Inspection is currently feasible.
    pub const CAN_INSPECT: u32 = 6;
    /// Escalation is mandatory.
    pub const MUST_ESCALATE: u32 = 7;
}

/// Field-pack bit-namespace re-exports (Phase 12, additive).
///
/// Packs allocate posture/context bits within their reserved band; these
/// re-exports surface the band ranges from `crate::packs::bits` without
/// pulling the full pack tree into the multimodal namespace.
///
/// Band layout: 0–15 core, 16–31 lifestyle, 32–47 edge, 48–55 enterprise,
/// 56–63 dev.
pub mod pack_bits {
    pub use crate::packs::bits::{
        CORE_RANGE, DEV_RANGE, EDGE_RANGE, ENTERPRISE_RANGE, LIFESTYLE_RANGE,
    };
}

/// Multimodal posture bundle from the trusted local interpreter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PostureBundle {
    /// Bitmask of [`PostureBit`] entries set by the interpreter.
    pub posture_mask: u64,
    /// Confidence in the posture interpretation (0-255).
    pub confidence: u8,
}

/// Local cognition context — expectation, risk, affordance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContextBundle {
    /// Bits from [`ContextBit`] indicating expected events.
    pub expectation_mask: u64,
    /// Bits from [`ContextBit`] indicating elevated risks.
    pub risk_mask: u64,
    /// Bits from [`ContextBit`] indicating available affordances.
    pub affordance_mask: u64,
}

impl PostureBundle {
    /// True iff `bit` is set in the posture mask.
    #[must_use]
    pub const fn has(&self, bit: u32) -> bool {
        (self.posture_mask >> bit) & 1 == 1
    }
}

impl ContextBundle {
    /// True iff `bit` is set in any of expectation/risk/affordance masks.
    #[must_use]
    pub const fn has_any(&self, bit: u32) -> bool {
        let m = 1u64 << bit;
        (self.expectation_mask & m) != 0
            || (self.risk_mask & m) != 0
            || (self.affordance_mask & m) != 0
    }

    /// True iff `bit` is set in `risk_mask`.
    #[must_use]
    pub const fn risk_has(&self, bit: u32) -> bool {
        (self.risk_mask >> bit) & 1 == 1
    }

    /// True iff `bit` is set in `expectation_mask`.
    #[must_use]
    pub const fn expect_has(&self, bit: u32) -> bool {
        (self.expectation_mask >> bit) & 1 == 1
    }

    /// True iff `bit` is set in `affordance_mask`.
    #[must_use]
    pub const fn afford_has(&self, bit: u32) -> bool {
        (self.affordance_mask >> bit) & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_has_isolates_risk_channel() {
        let ctx = ContextBundle {
            expectation_mask: 1u64 << ContextBit::PACKAGE_EXPECTED,
            risk_mask: 1u64 << ContextBit::THEFT_RISK,
            affordance_mask: 1u64 << ContextBit::CAN_INSPECT,
        };
        assert!(ctx.risk_has(ContextBit::THEFT_RISK));
        assert!(!ctx.risk_has(ContextBit::PACKAGE_EXPECTED));
        assert!(!ctx.risk_has(ContextBit::CAN_INSPECT));
    }

    #[test]
    fn expect_has_isolates_expectation_channel() {
        let ctx = ContextBundle {
            expectation_mask: 1u64 << ContextBit::PARTNER_DUE,
            risk_mask: 1u64 << ContextBit::PARTNER_DUE,
            affordance_mask: 0,
        };
        // expect_has only reads expectation_mask.
        assert!(ctx.expect_has(ContextBit::PARTNER_DUE));
        assert!(!ctx.expect_has(ContextBit::CAN_RETRIEVE_NOW));
    }

    #[test]
    fn afford_has_isolates_affordance_channel() {
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 0,
            affordance_mask: 1u64 << ContextBit::CAN_RETRIEVE_NOW,
        };
        assert!(ctx.afford_has(ContextBit::CAN_RETRIEVE_NOW));
        assert!(!ctx.afford_has(ContextBit::CAN_INSPECT));
    }

    #[test]
    fn posture_has_returns_true_for_set_bit() {
        let p = PostureBundle {
            posture_mask: 1u64 << PostureBit::ALERT,
            confidence: 100,
        };
        assert!(p.has(PostureBit::ALERT));
        assert!(!p.has(PostureBit::CALM));
    }
}
