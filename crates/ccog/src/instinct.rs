//! Autonomic Instinct response classes (Phase 6).
//!
//! Maps closed `O*` (compiled snapshot + posture + context) to a single
//! right-sized response class. The decision lattice reads the full closed
//! cognition surface — predicate masks from the snapshot, multimodal posture
//! bits from the trusted local interpreter, and local context (expectation,
//! risk, affordance) from the surrounding cognition space.

use crate::compiled::CompiledFieldSnapshot;
use crate::compiled_hook::{compute_present_mask, Predicate};
use crate::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};

/// Right-sized response class — the action the cognition surface admits.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum AutonomicInstinct {
    /// Known harmless event — return to baseline.
    Settle,
    /// Expected package/delivery — retrieve now.
    Retrieve,
    /// Unknown but low-threat — inspect.
    Inspect,
    /// Missing evidence — request clarification.
    Ask,
    /// Action does not belong — refuse the transition.
    Refuse,
    /// Persistent unresolved disturbance — escalate.
    Escalate,
    /// No-op. Default — safest fallback when no other variant applies.
    #[default]
    Ignore,
}

/// Select a single response class from the closed cognition surface.
///
/// The decision lattice is precedence-ordered: SETTLED posture closes the
/// loop first; risk overrides expectation; expectation+affordance+cadence
/// drive Retrieve; absent risks but missing evidence yields Ask; theft +
/// alert without affordance yields Refuse; ALERT/ENGAGED with inspect
/// affordance yields Inspect; calm baseline with no expectations yields
/// Ignore; default falls back to Ask.
///
/// `_v0` denotes lattice version one — the structure may extend with new
/// posture/context bits in subsequent versions, but the precedence ordering
/// is stable for `v0` consumers.
#[inline]
pub fn select_instinct_v0_with_reason(
    snap: &CompiledFieldSnapshot,
    posture: &PostureBundle,
    ctx: &ContextBundle,
) -> (AutonomicInstinct, &'static str) {
    let present = compute_present_mask(snap);

    if posture.has(PostureBit::SETTLED) {
        return (AutonomicInstinct::Settle, "settled posture");
    }
    if ctx.risk_has(ContextBit::MUST_ESCALATE) {
        return (AutonomicInstinct::Escalate, "must escalate risk");
    }
    if ctx.risk_has(ContextBit::SAFETY_RISK) && !ctx.afford_has(ContextBit::CAN_INSPECT) {
        return (AutonomicInstinct::Escalate, "safety risk without inspect affordance");
    }
    if ctx.expect_has(ContextBit::PACKAGE_EXPECTED)
        && ctx.afford_has(ContextBit::CAN_RETRIEVE_NOW)
        && (posture.has(PostureBit::CADENCE_DELIVERY) || posture.has(PostureBit::ORIENTED_TO_ENTRY))
    {
        return (AutonomicInstinct::Retrieve, "package expected with retrieve affordance");
    }
    if ctx.expect_has(ContextBit::PARTNER_DUE) && posture.has(PostureBit::CADENCE_PARTNER) {
        return (AutonomicInstinct::Settle, "partner due with partner cadence");
    }
    if (present & (1u64 << Predicate::DD_MISSING_PROV_VALUE)) != 0 {
        return (AutonomicInstinct::Ask, "missing evidence (DD missing prov:value)");
    }
    if ctx.risk_has(ContextBit::THEFT_RISK) && posture.has(PostureBit::ALERT) {
        return (AutonomicInstinct::Refuse, "theft risk under alert posture");
    }
    if ctx.afford_has(ContextBit::CAN_INSPECT)
        && (posture.has(PostureBit::ALERT) || posture.has(PostureBit::ENGAGED))
    {
        return (AutonomicInstinct::Inspect, "can inspect under alert/engaged posture");
    }
    if posture.has(PostureBit::CALM) && ctx.expectation_mask == 0 && ctx.risk_mask == 0 {
        return (AutonomicInstinct::Ignore, "calm baseline");
    }
    (AutonomicInstinct::Ask, "default fallback")
}

/// Wrapper over `select_instinct_v0_with_reason` returning only the `AutonomicInstinct`.
#[inline]
pub fn select_instinct_v0(
    snap: &CompiledFieldSnapshot,
    posture: &PostureBundle,
    ctx: &ContextBundle,
) -> AutonomicInstinct {
    select_instinct_v0_with_reason(snap, posture, ctx).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldContext;

    fn empty_snap() -> CompiledFieldSnapshot {
        let f = FieldContext::new("t");
        CompiledFieldSnapshot::from_field(&f).expect("snapshot")
    }

    fn dd_missing_snap() -> CompiledFieldSnapshot {
        let mut f = FieldContext::new("t");
        f.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )
        .expect("load");
        CompiledFieldSnapshot::from_field(&f).expect("snapshot")
    }

    #[test]
    fn settled_yields_settle() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::SETTLED,
            confidence: 200,
        };
        assert_eq!(
            select_instinct_v0(&snap, &posture, &ContextBundle::default()),
            AutonomicInstinct::Settle
        );
    }

    #[test]
    fn must_escalate_overrides_other_signals() {
        let snap = empty_snap();
        // Even with PACKAGE_EXPECTED + CAN_RETRIEVE_NOW + CADENCE_DELIVERY,
        // MUST_ESCALATE wins.
        let posture = PostureBundle {
            posture_mask: (1u64 << PostureBit::CADENCE_DELIVERY) | (1u64 << PostureBit::ORIENTED_TO_ENTRY),
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 1u64 << ContextBit::PACKAGE_EXPECTED,
            risk_mask: 1u64 << ContextBit::MUST_ESCALATE,
            affordance_mask: 1u64 << ContextBit::CAN_RETRIEVE_NOW,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Escalate);
    }

    #[test]
    fn safety_risk_without_inspect_affordance_escalates() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ALERT,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << ContextBit::SAFETY_RISK,
            affordance_mask: 0,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Escalate);
    }

    #[test]
    fn package_expected_plus_delivery_cadence_yields_retrieve() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::CADENCE_DELIVERY,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 1u64 << ContextBit::PACKAGE_EXPECTED,
            risk_mask: 0,
            affordance_mask: 1u64 << ContextBit::CAN_RETRIEVE_NOW,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Retrieve);
    }

    #[test]
    fn partner_due_plus_cadence_yields_settle() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::CADENCE_PARTNER,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 1u64 << ContextBit::PARTNER_DUE,
            risk_mask: 0,
            affordance_mask: 0,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Settle);
    }

    #[test]
    fn missing_evidence_yields_ask() {
        let snap = dd_missing_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ALERT,
            confidence: 200,
        };
        assert_eq!(
            select_instinct_v0(&snap, &posture, &ContextBundle::default()),
            AutonomicInstinct::Ask
        );
    }

    #[test]
    fn theft_risk_with_alert_yields_refuse() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ALERT,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << ContextBit::THEFT_RISK,
            affordance_mask: 0,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Refuse);
    }

    #[test]
    fn inspect_affordance_with_engaged_yields_inspect() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ENGAGED,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 0,
            affordance_mask: 1u64 << ContextBit::CAN_INSPECT,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Inspect);
    }

    #[test]
    fn calm_with_no_signals_yields_ignore() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::CALM,
            confidence: 200,
        };
        assert_eq!(
            select_instinct_v0(&snap, &posture, &ContextBundle::default()),
            AutonomicInstinct::Ignore
        );
    }

    #[test]
    fn default_falls_back_to_ask() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ORIENTED_INTERIOR,
            confidence: 200,
        };
        // No matching condition → default Ask.
        assert_eq!(
            select_instinct_v0(&snap, &posture, &ContextBundle::default()),
            AutonomicInstinct::Ask
        );
    }

    #[test]
    fn settled_overrides_must_escalate() {
        // SETTLED wins over MUST_ESCALATE (resolution beats escalation).
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::SETTLED,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << ContextBit::MUST_ESCALATE,
            affordance_mask: 0,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Settle);
    }

    #[test]
    fn must_escalate_overrides_retrieve() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::CADENCE_DELIVERY,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 1u64 << ContextBit::PACKAGE_EXPECTED,
            risk_mask: 1u64 << ContextBit::MUST_ESCALATE,
            affordance_mask: 1u64 << ContextBit::CAN_RETRIEVE_NOW,
        };
        assert_eq!(select_instinct_v0(&snap, &posture, &ctx), AutonomicInstinct::Escalate);
    }
}
