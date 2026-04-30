//! Enterprise field pack.
//!
//! Targets enterprise governance cognition: gap-evidence requests, transition
//! admittance, owner routing, compliance escalation. Every action emits a
//! `prov:Activity` plus `prov:wasInformedBy` and `prov:used` triples — the
//! minimum PROV shape for an auditable enterprise step. Admits all canonical
//! breeds because enterprise contexts can fire any of them.

use crate::bark_artifact::BarkSlot;
use crate::ccog_const_assert;
use crate::compiled::CompiledFieldSnapshot;
use crate::construct8::Construct8;
use crate::instinct::AutonomicInstinct;
use crate::multimodal::{ContextBundle, PostureBundle};
use crate::packs::bits::ENTERPRISE_RANGE;
use crate::packs::FieldPack;
use crate::verdict::Breed;
use anyhow::Result;
use oxigraph::model::{NamedNode, Term, Triple};

/// Enterprise pack bits — local within the [`ENTERPRISE_RANGE`] band.
#[allow(non_snake_case)]
pub mod EnterpriseBit {
    /// Gap evidence has been requested.
    pub const GAP_REQUESTED: u32 = 48;
    /// Transition admittance is pending.
    pub const TRANSITION_PENDING: u32 = 49;
    /// Owner routing is in progress.
    pub const ROUTE_PENDING: u32 = 50;
    /// Compliance escalation has fired.
    pub const COMPLIANCE_HOT: u32 = 51;
}

ccog_const_assert!(EnterpriseBit::GAP_REQUESTED >= ENTERPRISE_RANGE.start);
ccog_const_assert!(EnterpriseBit::COMPLIANCE_HOT < ENTERPRISE_RANGE.end);

/// Enterprise pack handle (zero-sized).
pub struct EnterprisePack;

impl FieldPack for EnterprisePack {
    const NAME: &'static str = "enterprise";
    const ONTOLOGY_PROFILE: &'static [&'static str] = &[
        "http://www.w3.org/ns/prov#",
        "http://www.w3.org/ns/shacl#",
        "https://schema.org/",
        "urn:blake3:",
        "urn:ccog:vocab:",
    ];
    const ADMITTED_BREEDS: &'static [Breed] = &[
        Breed::Eliza,
        Breed::Mycin,
        Breed::Strips,
        Breed::Shrdlu,
        Breed::Prolog,
        Breed::Hearsay,
        Breed::Dendral,
    ];
    const POSTURE_RANGE: core::ops::Range<u32> = ENTERPRISE_RANGE;
    const CONTEXT_RANGE: core::ops::Range<u32> = ENTERPRISE_RANGE;

    fn builtins() -> &'static [BarkSlot] {
        BUILTINS
    }
}

/// Static const table of Enterprise pack bark slots.
pub static BUILTINS: &[BarkSlot] = &[
    BarkSlot {
        name: "gap_request_evidence",
        require_mask: 0,
        act: act_gap_request_evidence,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "transition_admit",
        require_mask: 0,
        act: act_transition_admit,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "route_to_owner",
        require_mask: 0,
        act: act_route_to_owner,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "escalate_compliance",
        require_mask: 0,
        act: act_escalate_compliance,
        emit_receipt: true,
        predecessor_mask: 0,
    },
];

fn prov_full_activity(tag: &[u8]) -> Result<Construct8> {
    let h = blake3::hash(tag);
    let activity = NamedNode::new(format!("urn:blake3:{}", h.to_hex()))?;
    let informed_by = NamedNode::new(format!(
        "urn:blake3:{}",
        blake3::hash(b"enterprise/source").to_hex()
    ))?;
    let used = NamedNode::new(format!(
        "urn:blake3:{}",
        blake3::hash(b"enterprise/evidence").to_hex()
    ))?;

    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let prov_activity = NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let p_was_informed_by = NamedNode::new("http://www.w3.org/ns/prov#wasInformedBy")?;
    let p_used = NamedNode::new("http://www.w3.org/ns/prov#used")?;

    let act_term: Term = prov_activity.into();
    let informed_term: Term = informed_by.into();
    let used_term: Term = used.into();

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(activity.clone(), rt, act_term));
    let _ = delta.push(Triple::new(activity.clone(), p_was_informed_by, informed_term));
    let _ = delta.push(Triple::new(activity, p_used, used_term));
    Ok(delta)
}

fn act_gap_request_evidence(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    prov_full_activity(b"enterprise/gap_request_evidence")
}

fn act_transition_admit(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    prov_full_activity(b"enterprise/transition_admit")
}

fn act_route_to_owner(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    prov_full_activity(b"enterprise/route_to_owner")
}

fn act_escalate_compliance(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    prov_full_activity(b"enterprise/escalate_compliance")
}

/// Bias wrapper: enterprise pack is governance-neutral — pass through.
#[must_use]
pub fn select_instinct(
    snap: &CompiledFieldSnapshot,
    posture: &PostureBundle,
    ctx: &ContextBundle,
) -> AutonomicInstinct {
    crate::instinct::select_instinct_v0(snap, posture, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldContext;

    #[test]
    fn every_act_emits_was_informed_by_and_used() {
        let f = FieldContext::new("t");
        let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
        for slot in BUILTINS {
            let delta = (slot.act)(&snap).expect("act");
            let nt = delta.to_ntriples();
            assert!(
                nt.contains("prov#wasInformedBy"),
                "enterprise slot {} missing wasInformedBy",
                slot.name
            );
            assert!(
                nt.contains("prov#used"),
                "enterprise slot {} missing prov:used",
                slot.name
            );
        }
    }
}
