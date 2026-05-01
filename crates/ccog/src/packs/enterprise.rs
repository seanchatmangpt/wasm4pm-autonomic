//! Enterprise field pack.
//!
//! Targets enterprise governance cognition: gap-evidence requests, transition
//! admittance, owner routing, compliance escalation. Every action emits a
//! `prov:Activity` plus `prov:wasInformedBy` and `prov:used` triples — the
//! minimum PROV shape for an auditable enterprise step. Admits all canonical
//! breeds because enterprise contexts can fire any of them.

use crate::bark_artifact::BarkSlot;
use crate::ccog_const_assert;
use crate::construct8::{Construct8, Triple};
use crate::instinct::AutonomicInstinct;
use crate::packs::bits::ENTERPRISE_RANGE;
use crate::packs::FieldPack;
use crate::runtime::cog8::{
    BreedId, Cog8Edge, Cog8Row, CollapseFn, EdgeId, EdgeKind, FieldId, GroupId, Instinct, NodeId,
    PackId, Powl8Instr, Powl8Op, RuleId,
};
use crate::runtime::ClosedFieldContext;
use crate::verdict::Breed;
use anyhow::Result;

/// Enterprise pack numeric ID.
pub const PACK_ID: PackId = PackId(3);

/// Enterprise pack bits — local within the [`ENTERPRISE_RANGE`] band.
#[allow(non_snake_case)]
pub mod Bit {
    /// Gap evidence has been requested.
    pub const GAP_REQUESTED: u32 = 48;
    /// Transition admittance is pending.
    pub const TRANSITION_PENDING: u32 = 49;
    /// Owner routing is in progress.
    pub const ROUTE_PENDING: u32 = 50;
    /// Compliance escalation has fired.
    pub const COMPLIANCE_HOT: u32 = 51;
}

ccog_const_assert!(Bit::GAP_REQUESTED >= ENTERPRISE_RANGE.start);
ccog_const_assert!(Bit::COMPLIANCE_HOT < ENTERPRISE_RANGE.end);

/// COG8 closure nodes for Enterprise governance (PRD v0.5).
pub static COG8_NODES: &[Cog8Row] = &[
    // Node 0: Gap evidence requested.
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(3),
        rule_id: RuleId(48),
        breed_id: BreedId(Breed::Mycin as u8),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [
            FieldId(48),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::GAP_REQUESTED,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 10,
    },
    // Node 1: Transition admittance pending.
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(3),
        rule_id: RuleId(49),
        breed_id: BreedId(Breed::Prolog as u8),
        collapse_fn: CollapseFn::RelationalProof,
        var_ids: [
            FieldId(49),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::TRANSITION_PENDING,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Inspect,
        priority: 10,
    },
    // Node 2: Owner routing in progress.
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(3),
        rule_id: RuleId(50),
        breed_id: BreedId(Breed::Mycin as u8),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [
            FieldId(50),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::ROUTE_PENDING,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Inspect,
        priority: 10,
    },
    // Node 3: Compliance escalation hot.
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(3),
        rule_id: RuleId(51),
        breed_id: BreedId(Breed::Mycin as u8),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [
            FieldId(51),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::COMPLIANCE_HOT,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Escalate,
        priority: 20,
    },
];

/// POWL8 topology edges for Enterprise governance (PRD v0.5).
pub static COG8_EDGES: &[Cog8Edge] = &[
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(0),
            guard_mask: 0,
            effect_mask: 1u64 << 0,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(1),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::RelationalProof,
            node_id: NodeId(1),
            edge_id: EdgeId(1),
            guard_mask: 0,
            effect_mask: 1u64 << 1,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(2),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(2),
            edge_id: EdgeId(2),
            guard_mask: 0,
            effect_mask: 1u64 << 2,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(3),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(3),
            edge_id: EdgeId(3),
            guard_mask: 0,
            effect_mask: 1u64 << 3,
        },
    },
];

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
    let activity = format!("urn:blake3:{}", h.to_hex());
    let informed_by = format!("urn:blake3:{}", blake3::hash(b"enterprise/source").to_hex());
    let used = format!(
        "urn:blake3:{}",
        blake3::hash(b"enterprise/evidence").to_hex()
    );

    let rt = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let prov_activity = "http://www.w3.org/ns/prov#Activity";
    let p_was_informed_by = "http://www.w3.org/ns/prov#wasInformedBy";
    let p_used = "http://www.w3.org/ns/prov#used";

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::from_strings(&activity, rt, prov_activity));
    let _ = delta.push(Triple::from_strings(
        &activity,
        p_was_informed_by,
        &informed_by,
    ));
    let _ = delta.push(Triple::from_strings(&activity, p_used, &used));
    Ok(delta)
}

fn act_gap_request_evidence(_context: &ClosedFieldContext) -> Result<Construct8> {
    prov_full_activity(b"enterprise/gap_request_evidence")
}

fn act_transition_admit(_context: &ClosedFieldContext) -> Result<Construct8> {
    prov_full_activity(b"enterprise/transition_admit")
}

fn act_route_to_owner(_context: &ClosedFieldContext) -> Result<Construct8> {
    prov_full_activity(b"enterprise/route_to_owner")
}

fn act_escalate_compliance(_context: &ClosedFieldContext) -> Result<Construct8> {
    prov_full_activity(b"enterprise/escalate_compliance")
}

/// Bias wrapper: enterprise pack is governance-neutral — pass through.
#[must_use]
pub fn select_instinct(context: &ClosedFieldContext) -> AutonomicInstinct {
    crate::instinct::select_instinct_v0(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBundle, PostureBundle};
    use crate::packs::TierMasks;

    #[test]
    fn every_act_emits_was_informed_by_and_used() {
        let f = FieldContext::new("t");
        let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };
        for slot in BUILTINS {
            let delta = (slot.act)(&context).expect("act");
            let nt = delta.to_ntriples();
            // prov:wasInformedBy hash (u16)
            let h_informed = format!(
                "{:04x}",
                crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#wasInformedBy".as_bytes())
                    as u16
            );
            assert!(
                nt.contains(&h_informed),
                "enterprise slot {} missing wasInformedBy ({})",
                slot.name,
                h_informed
            );
            // prov:used hash (u16)
            let h_used = format!(
                "{:04x}",
                crate::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#used".as_bytes()) as u16
            );
            assert!(
                nt.contains(&h_used),
                "enterprise slot {} missing prov:used ({})",
                slot.name,
                h_used
            );
        }
    }
}
