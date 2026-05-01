//! Dev / Agent Governance field pack.
//!
//! Targets developer-facing agent governance: boundary-test guards, nightly
//! feature checks, mask-domain audits, CLAUDE.md revision requests. This
//! pack carries a hard bias: any decision that would otherwise be `Refuse`
//! or `Escalate` is clamped to `Ask` — agents must never auto-merge or
//! auto-escalate dev changes; they must surface them for human review.
//!
//! Admits MYCIN (evidence gap), DENDRAL (provenance chain), Prolog
//! (transitive proof) — all are appropriate for code-governance contexts.

use crate::bark_artifact::BarkSlot;
use crate::ccog_const_assert;
use crate::construct8::{Construct8, ObjectId, PredicateId, Triple};
use crate::instinct::AutonomicInstinct;
use crate::packs::bits::DEV_RANGE;
use crate::packs::FieldPack;
use crate::runtime::cog8::{
    BreedId, Cog8Edge, Cog8Row, CollapseFn, EdgeId, EdgeKind, FieldId, GroupId, Instinct, NodeId,
    PackId, Powl8Instr, Powl8Op, RuleId,
};
use crate::runtime::ClosedFieldContext;
use crate::utils::dense::fnv1a_64;
use crate::verdict::Breed;
use anyhow::Result;

/// Dev / Agent Governance pack numeric ID.
pub const PACK_ID: PackId = PackId(4);

/// Dev pack bits — local within the [`DEV_RANGE`] band.
#[allow(non_snake_case)]
pub mod Bit {
    /// A boundary test failed.
    pub const BOUNDARY_FAILED: u32 = 56;
    /// A nightly feature is in use.
    pub const NIGHTLY_FEATURE: u32 = 57;
    /// A mask domain is unclear and needs audit.
    pub const MASK_DOMAIN_UNCLEAR: u32 = 58;
    /// CLAUDE.md is stale or missing context.
    pub const CLAUDE_MD_STALE: u32 = 59;
}

ccog_const_assert!(Bit::BOUNDARY_FAILED >= DEV_RANGE.start);
ccog_const_assert!(Bit::CLAUDE_MD_STALE < DEV_RANGE.end);

/// COG8 closure nodes for Dev / Agent Governance (PRD v0.5).
pub static COG8_NODES: &[Cog8Row] = &[
    // Node 0: Start marker (Silent entry)
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(0),
        rule_id: RuleId(0),
        breed_id: BreedId(Breed::CompiledHook as u8),
        collapse_fn: CollapseFn::None,
        var_ids: [FieldId(0); 8],
        required_mask: 0,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ignore,
        priority: 0,
    },
    // Node 1: boundary_test_guard
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(4),
        rule_id: RuleId(Bit::BOUNDARY_FAILED as u16),
        breed_id: BreedId(Breed::Mycin as u8),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [
            FieldId(Bit::BOUNDARY_FAILED as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::BOUNDARY_FAILED,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 10,
    },
    // Node 2: nightly_feature_check
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(4),
        rule_id: RuleId(Bit::NIGHTLY_FEATURE as u16),
        breed_id: BreedId(Breed::Dendral as u8),
        collapse_fn: CollapseFn::Reconstruction,
        var_ids: [
            FieldId(Bit::NIGHTLY_FEATURE as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::NIGHTLY_FEATURE,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 10,
    },
    // Node 3: mask_domain_audit
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(4),
        rule_id: RuleId(Bit::MASK_DOMAIN_UNCLEAR as u16),
        breed_id: BreedId(Breed::Prolog as u8),
        collapse_fn: CollapseFn::RelationalProof,
        var_ids: [
            FieldId(Bit::MASK_DOMAIN_UNCLEAR as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::MASK_DOMAIN_UNCLEAR,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 10,
    },
    // Node 4: claude_md_revise_request
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(4),
        rule_id: RuleId(Bit::CLAUDE_MD_STALE as u16),
        breed_id: BreedId(Breed::Mycin as u8),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [
            FieldId(Bit::CLAUDE_MD_STALE as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::CLAUDE_MD_STALE,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 10,
    },
];

/// POWL8 topology edges for Dev / Agent Governance (PRD v0.5).
pub static COG8_EDGES: &[Cog8Edge] = &[
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(1),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
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
            collapse_fn: CollapseFn::Reconstruction,
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
            collapse_fn: CollapseFn::RelationalProof,
            node_id: NodeId(3),
            edge_id: EdgeId(3),
            guard_mask: 0,
            effect_mask: 1u64 << 3,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(4),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(4),
            edge_id: EdgeId(4),
            guard_mask: 0,
            effect_mask: 1u64 << 4,
        },
    },
];

/// Dev / Agent Governance pack handle (zero-sized).
pub struct DevPack;

impl FieldPack for DevPack {
    const NAME: &'static str = "dev";
    const ONTOLOGY_PROFILE: &'static [&'static str] = &[
        "http://www.w3.org/ns/prov#",
        "https://schema.org/",
        "urn:blake3:",
        "urn:ccog:vocab:",
    ];
    const ADMITTED_BREEDS: &'static [Breed] = &[Breed::Mycin, Breed::Dendral, Breed::Prolog];
    const POSTURE_RANGE: core::ops::Range<u32> = DEV_RANGE;
    const CONTEXT_RANGE: core::ops::Range<u32> = DEV_RANGE;

    fn builtins() -> &'static [BarkSlot] {
        BUILTINS
    }
}

/// Static const table of Dev pack bark slots.
pub static BUILTINS: &[BarkSlot] = &[
    BarkSlot {
        name: "boundary_test_guard",
        require_mask: 0,
        act: act_boundary_test_guard,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "nightly_feature_check",
        require_mask: 0,
        act: act_nightly_feature_check,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "mask_domain_audit",
        require_mask: 0,
        act: act_mask_domain_audit,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "claude_md_revise_request",
        require_mask: 0,
        act: act_claude_md_revise_request,
        emit_receipt: true,
        predecessor_mask: 0,
    },
];

/// Dev pack actions emit a `schema:AskAction` plus a PROV activity — never a
/// `schema:AcceptAction` or `prov:Communication`. Auto-merge is structurally
/// impossible at the artifact level.
fn ask_action(tag: &[u8]) -> Result<Construct8> {
    let h = blake3::hash(tag);
    let activity_uri = format!("urn:blake3:{}", h.to_hex());
    let rt_uri = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let prov_activity_uri = "http://www.w3.org/ns/prov#Activity";
    let ask_action_uri = "https://schema.org/AskAction";

    let subject = ObjectId(fnv1a_64(activity_uri.as_bytes()) as u32);
    let rt_predicate = PredicateId(fnv1a_64(rt_uri.as_bytes()) as u16);
    let prov_activity_obj = ObjectId(fnv1a_64(prov_activity_uri.as_bytes()) as u32);
    let ask_action_obj = ObjectId(fnv1a_64(ask_action_uri.as_bytes()) as u32);

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(subject, rt_predicate, prov_activity_obj));
    let _ = delta.push(Triple::new(subject, rt_predicate, ask_action_obj));
    Ok(delta)
}

fn act_boundary_test_guard(_context: &ClosedFieldContext) -> Result<Construct8> {
    ask_action(b"dev/boundary_test_guard")
}

fn act_nightly_feature_check(_context: &ClosedFieldContext) -> Result<Construct8> {
    ask_action(b"dev/nightly_feature_check")
}

fn act_mask_domain_audit(_context: &ClosedFieldContext) -> Result<Construct8> {
    ask_action(b"dev/mask_domain_audit")
}

fn act_claude_md_revise_request(_context: &ClosedFieldContext) -> Result<Construct8> {
    ask_action(b"dev/claude_md_revise_request")
}

/// Bias wrapper: clamp `Refuse` and `Escalate` to `Ask` — dev pack actions
/// must always surface for human review, never auto-merge or auto-block.
#[must_use]
pub fn select_instinct(context: &ClosedFieldContext) -> AutonomicInstinct {
    let base = crate::instinct::select_instinct_v0(context);
    match base {
        AutonomicInstinct::Refuse | AutonomicInstinct::Escalate => AutonomicInstinct::Ask,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
    use crate::packs::TierMasks;

    fn empty_snap() -> CompiledFieldSnapshot {
        let f = FieldContext::new("t");
        CompiledFieldSnapshot::from_field(&f).expect("snap")
    }

    #[test]
    fn refuse_is_clamped_to_ask() {
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
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };
        assert_eq!(select_instinct(&context), AutonomicInstinct::Ask);
    }

    #[test]
    fn escalate_is_clamped_to_ask() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ALERT,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << ContextBit::MUST_ESCALATE,
            affordance_mask: 0,
        };
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx,
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };
        assert_eq!(select_instinct(&context), AutonomicInstinct::Ask);
    }

    #[test]
    fn settle_passes_through() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::SETTLED,
            confidence: 200,
        };
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };
        assert_eq!(select_instinct(&context), AutonomicInstinct::Settle);
    }
}
