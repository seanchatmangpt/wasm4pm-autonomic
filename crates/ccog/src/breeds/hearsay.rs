//! Hearsay-II breed: blackboard fusion of HookOutcomes into a PackPosture.

use anyhow::Result;
use crate::field::FieldContext;
use crate::hooks::HookOutcome;
use crate::verdict::PackPosture;

/// Fuse a slice of `HookOutcome`s into a single `PackPosture`.
///
/// Counts outcomes carrying receipts (confirmed signals) and maps the count
/// to a posture band: 0->Calm, 1->Alert, 2-3->Engaged, 4+->Settled.
/// Outcomes with names containing "missing_evidence" escalate by one band
/// (Calm->Alert, Alert->Engaged), never downgrade.
pub fn fuse_posture(outcomes: &[HookOutcome], _field: &FieldContext) -> Result<PackPosture> {
    let confirmed = outcomes.iter().filter(|o| o.receipt.is_some()).count();
    let base = match confirmed {
        0 => PackPosture::Calm,
        1 => PackPosture::Alert,
        2..=3 => PackPosture::Engaged,
        _ => PackPosture::Settled,
    };
    let escalate = outcomes.iter().any(|o| o.hook_name.contains("missing_evidence"));
    let posture = if escalate {
        match base {
            PackPosture::Calm => PackPosture::Alert,
            PackPosture::Alert => PackPosture::Engaged,
            other => other,
        }
    } else {
        base
    };
    Ok(posture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct8::Construct8;
    use crate::receipt::Receipt;
    use crate::graph::GraphIri;
    use chrono::Utc;

    /// Deterministic test-only constructor for a single `HookOutcome`.
    fn make_outcome(name: &'static str, with_receipt: bool) -> HookOutcome {
        let receipt = if with_receipt {
            Some(Receipt::new(
                GraphIri::from_iri("urn:test:act:1").unwrap(),
                Receipt::blake3_hex(b""),
                Utc::now(),
            ))
        } else { None };
        HookOutcome { hook_name: name, delta: Construct8::empty(), receipt }
    }

    #[test]
    fn zero_outcomes_is_calm() {
        let f = FieldContext::new("t");
        assert_eq!(fuse_posture(&[], &f).unwrap(), PackPosture::Calm);
    }
    #[test]
    fn one_confirmed_is_alert() {
        let f = FieldContext::new("t");
        let o = vec![make_outcome("phrase_binding", true)];
        assert_eq!(fuse_posture(&o, &f).unwrap(), PackPosture::Alert);
    }
    #[test]
    fn two_confirmed_is_engaged() {
        let f = FieldContext::new("t");
        let o = vec![make_outcome("a", true), make_outcome("b", true)];
        assert_eq!(fuse_posture(&o, &f).unwrap(), PackPosture::Engaged);
    }
    #[test]
    fn four_confirmed_is_settled() {
        let f = FieldContext::new("t");
        let o = vec![make_outcome("a", true), make_outcome("b", true), make_outcome("c", true), make_outcome("d", true)];
        assert_eq!(fuse_posture(&o, &f).unwrap(), PackPosture::Settled);
    }
    #[test]
    fn missing_evidence_escalates() {
        let f = FieldContext::new("t");
        let o = vec![make_outcome("missing_evidence", true)];
        assert_eq!(fuse_posture(&o, &f).unwrap(), PackPosture::Engaged);
    }
    #[test]
    fn no_receipt_does_not_count() {
        let f = FieldContext::new("t");
        let o = vec![make_outcome("a", false), make_outcome("b", true)];
        assert_eq!(fuse_posture(&o, &f).unwrap(), PackPosture::Alert);
    }
}
