//! Phase 7.1 — Lifestyle Overlap pack (runtime, K-tier).
//!
//! Five-field vertical slice — `Routine ⊕ Capacity ⊕ Safety ⊕ Evidence ⊕
//! Meaning` — proving the universal Lifestyle Redesign engine:
//!
//! ```text
//! routine pressure
//! + biological capacity
//! + safety boundary
//! + evidence gap
//! + meaningful occupation
//! → canonical lattice response
//! ```
//!
//! Constitutional: packs MUST NOT fork the canonical
//! [`crate::instinct::AutonomicInstinct`] lattice. "Smallest version" /
//! "scale meaningful activity" semantics live in `matched_rule_id`, not
//! in a new response class.

use crate::instinct::AutonomicInstinct;
use crate::packs::{LoadedFieldPack, LoadedPackRule, LoadedRuleGroup};

// =============================================================================
// K1 bit positions — Routine / Capacity / Safety
// =============================================================================

/// K1 routine bits.
#[allow(non_snake_case)]
pub mod RoutineBit {
    /// A routine is currently due.
    pub const ROUTINE_DUE: u32 = 0;
    /// A routine has been missed.
    pub const ROUTINE_MISSED: u32 = 1;
    /// A routine has just been deferred.
    pub const ROUTINE_DEFERRED: u32 = 2;
}

/// K1 capacity bits (offset 16 within K1).
#[allow(non_snake_case)]
pub mod CapacityBit {
    /// Subject is fatigued.
    pub const FATIGUE_HIGH: u32 = 16;
    /// Subject has low executive capacity.
    pub const LOW_EXEC_CAPACITY: u32 = 17;
    /// Recovery / rest window is needed.
    pub const RECOVERY_NEEDED: u32 = 18;
}

/// K1 safety bits (offset 48 within K1).
#[allow(non_snake_case)]
pub mod SafetyBit {
    /// Driving-while-impaired risk.
    pub const DRIVING_RISK: u32 = 48;
    /// Medication overdue beyond the safe window.
    pub const MEDICATION_OVERDUE: u32 = 49;
    /// Acute distress / urgent health signal.
    pub const ACUTE_DISTRESS: u32 = 50;
}

// =============================================================================
// K2 bit positions — Meaning
// =============================================================================

/// K2 meaning bits.
#[allow(non_snake_case)]
pub mod MeaningBit {
    /// An identity-reinforcing / meaningful occupation is currently
    /// available (e.g. service tomorrow, family role, recovery practice).
    pub const IDENTITY_REINFORCING_AVAILABLE: u32 = 0;
}

// =============================================================================
// K3 bit positions — Evidence
// =============================================================================

/// K3 evidence bits (offset 38 within K3).
#[allow(non_snake_case)]
pub mod EvidenceBit {
    /// The system does not know whether the meal happened.
    pub const MEAL_EVIDENCE_MISSING: u32 = 38;
    /// The system does not know whether a routine completed.
    pub const ROUTINE_COMPLETION_UNCERTAIN: u32 = 39;
}

// =============================================================================
// Precedence ranks — Safety > Evidence > Capacity > Meaning > Routine
// =============================================================================

/// Group precedence: lower = earlier. Safety preempts everything else.
pub const PRECEDENCE_SAFETY: u32 = 10;
/// Evidence-gap takes precedence over capacity-driven softening — never
/// fabricate closure when reality is uncertain.
pub const PRECEDENCE_EVIDENCE: u32 = 20;
/// Capacity softens routine pressure when no safety/evidence overrides.
pub const PRECEDENCE_CAPACITY: u32 = 30;
/// Meaning preserves identity-reinforcing occupations.
pub const PRECEDENCE_MEANING: u32 = 40;
/// Routine fires only after every other field has had its say.
pub const PRECEDENCE_ROUTINE: u32 = 50;

/// Build the Phase 7.1 Lifestyle Overlap pack as a [`LoadedFieldPack`].
///
/// The pack ships five precedence groups in declared order. Callers
/// should run [`crate::packs::sort_groups_by_precedence`] after load to
/// guarantee ascending evaluation regardless of how the pack arrived.
///
/// Provenance fields (`name`, `digest_urn`) are filled by the caller
/// (`autoinstinct::compile`); this builder only ships the rule shape.
#[must_use]
pub fn build_lifestyle_overlap_pack(name: &str, digest_urn: &str) -> LoadedFieldPack {
    let safety = LoadedRuleGroup {
        id: "lifestyle.safety".to_string(),
        precedence_rank: PRECEDENCE_SAFETY,
        rules: vec![
            LoadedPackRule {
                id: "lifestyle.safety.driving_risk_refuses".to_string(),
                response: AutonomicInstinct::Refuse,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 1u64 << SafetyBit::DRIVING_RISK,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
            LoadedPackRule {
                id: "lifestyle.safety.medication_overdue_escalates".to_string(),
                response: AutonomicInstinct::Escalate,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 1u64 << SafetyBit::MEDICATION_OVERDUE,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
            LoadedPackRule {
                id: "lifestyle.safety.acute_distress_escalates".to_string(),
                response: AutonomicInstinct::Escalate,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 1u64 << SafetyBit::ACUTE_DISTRESS,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
        ],
    };

    let evidence = LoadedRuleGroup {
        id: "lifestyle.evidence".to_string(),
        precedence_rank: PRECEDENCE_EVIDENCE,
        rules: vec![
            LoadedPackRule {
                id: "lifestyle.evidence.missing_completion_asks".to_string(),
                response: AutonomicInstinct::Ask,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 0,
                require_k2_mask: 0,
                require_k3_mask: 1u64 << EvidenceBit::MEAL_EVIDENCE_MISSING,
            },
            LoadedPackRule {
                id: "lifestyle.evidence.routine_uncertain_inspects".to_string(),
                response: AutonomicInstinct::Inspect,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 0,
                require_k2_mask: 0,
                require_k3_mask: 1u64 << EvidenceBit::ROUTINE_COMPLETION_UNCERTAIN,
            },
        ],
    };

    let capacity = LoadedRuleGroup {
        id: "lifestyle.capacity".to_string(),
        precedence_rank: PRECEDENCE_CAPACITY,
        rules: vec![LoadedPackRule {
            id: "lifestyle.capacity.fatigue_softens_routine".to_string(),
            // Routine pressure under fatigue collapses to Ask, not
            // Refuse — the matched_rule_id renders "smallest version"
            // language at the UI layer.
            response: AutonomicInstinct::Ask,
            require_posture_mask: 0,
            require_expectation_mask: 0,
            require_risk_mask: 0,
            require_affordance_mask: 0,
            require_k1_mask: (1u64 << CapacityBit::FATIGUE_HIGH) | (1u64 << RoutineBit::ROUTINE_DUE),
            require_k2_mask: 0,
            require_k3_mask: 0,
        }],
    };

    let meaning = LoadedRuleGroup {
        id: "lifestyle.meaning".to_string(),
        precedence_rank: PRECEDENCE_MEANING,
        rules: vec![LoadedPackRule {
            id: "lifestyle.meaning.scale_meaningful_activity".to_string(),
            // Even under fatigue, identity-reinforcing activity is
            // preserved as Retrieve (smaller-version) rather than
            // erased. Response stays canonical; "scale" lives in id.
            response: AutonomicInstinct::Retrieve,
            require_posture_mask: 0,
            require_expectation_mask: 0,
            require_risk_mask: 0,
            require_affordance_mask: 0,
            require_k1_mask: 0,
            require_k2_mask: 1u64 << MeaningBit::IDENTITY_REINFORCING_AVAILABLE,
            require_k3_mask: 0,
        }],
    };

    let routine = LoadedRuleGroup {
        id: "lifestyle.routine".to_string(),
        precedence_rank: PRECEDENCE_ROUTINE,
        rules: vec![
            LoadedPackRule {
                id: "lifestyle.routine.due_asks".to_string(),
                response: AutonomicInstinct::Ask,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 1u64 << RoutineBit::ROUTINE_DUE,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
            LoadedPackRule {
                id: "lifestyle.routine.missed_inspects".to_string(),
                response: AutonomicInstinct::Inspect,
                require_posture_mask: 0,
                require_expectation_mask: 0,
                require_risk_mask: 0,
                require_affordance_mask: 0,
                require_k1_mask: 1u64 << RoutineBit::ROUTINE_MISSED,
                require_k2_mask: 0,
                require_k3_mask: 0,
            },
        ],
    };

    LoadedFieldPack {
        name: name.to_string(),
        ontology_profile: vec![
            "https://schema.org/".to_string(),
            "http://www.w3.org/ns/prov#".to_string(),
            "urn:ccog:vocab:".to_string(),
            "urn:blake3:".to_string(),
        ],
        rules: Vec::new(),
        mask_rules: Vec::new(),
        groups: vec![safety, evidence, capacity, meaning, routine],
        default_response: format!("{:?}", AutonomicInstinct::Ignore),
        digest_urn: digest_urn.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::{sort_groups_by_precedence, validate};

    #[test]
    fn pack_validates_clean() {
        let mut p = build_lifestyle_overlap_pack("test.lifestyle", "urn:blake3:fixture");
        sort_groups_by_precedence(&mut p);
        validate(&p).expect("Phase 7.1 lifestyle pack must validate");
    }

    #[test]
    fn precedence_ranks_are_distinct_and_ordered() {
        let p = build_lifestyle_overlap_pack("test.lifestyle", "urn:blake3:fixture");
        let ranks: Vec<u32> = p.groups.iter().map(|g| g.precedence_rank).collect();
        // Five groups, all distinct, all in declared ascending order.
        assert_eq!(ranks.len(), 5);
        for w in ranks.windows(2) {
            assert!(w[0] < w[1], "ranks not strictly ascending: {ranks:?}");
        }
    }

    #[test]
    fn safety_outranks_routine_in_precedence_table() {
        assert!(PRECEDENCE_SAFETY < PRECEDENCE_ROUTINE);
        assert!(PRECEDENCE_EVIDENCE < PRECEDENCE_CAPACITY);
        assert!(PRECEDENCE_CAPACITY < PRECEDENCE_MEANING);
        assert!(PRECEDENCE_MEANING < PRECEDENCE_ROUTINE);
    }
}
