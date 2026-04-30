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
use crate::compiled::CompiledFieldSnapshot;
use crate::construct8::Construct8;
use crate::instinct::AutonomicInstinct;
use crate::multimodal::{ContextBundle, PostureBundle};
use crate::packs::bits::DEV_RANGE;
use crate::packs::FieldPack;
use crate::verdict::Breed;
use anyhow::Result;
use oxigraph::model::{NamedNode, Term, Triple};

/// Dev pack bits — local within the [`DEV_RANGE`] band.
#[allow(non_snake_case)]
pub mod DevBit {
    /// A boundary test failed.
    pub const BOUNDARY_FAILED: u32 = 56;
    /// A nightly feature is in use.
    pub const NIGHTLY_FEATURE: u32 = 57;
    /// A mask domain is unclear and needs audit.
    pub const MASK_DOMAIN_UNCLEAR: u32 = 58;
    /// CLAUDE.md is stale or missing context.
    pub const CLAUDE_MD_STALE: u32 = 59;
}

ccog_const_assert!(DevBit::BOUNDARY_FAILED >= DEV_RANGE.start);
ccog_const_assert!(DevBit::CLAUDE_MD_STALE < DEV_RANGE.end);

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
    let activity = NamedNode::new(format!("urn:blake3:{}", h.to_hex()))?;
    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let prov_activity = NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let ask_action_iri = NamedNode::new("https://schema.org/AskAction")?;
    let activity_term: Term = prov_activity.into();
    let ask_term: Term = ask_action_iri.into();

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(activity.clone(), rt.clone(), activity_term));
    let _ = delta.push(Triple::new(activity, rt, ask_term));
    Ok(delta)
}

fn act_boundary_test_guard(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    ask_action(b"dev/boundary_test_guard")
}

fn act_nightly_feature_check(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    ask_action(b"dev/nightly_feature_check")
}

fn act_mask_domain_audit(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    ask_action(b"dev/mask_domain_audit")
}

fn act_claude_md_revise_request(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    ask_action(b"dev/claude_md_revise_request")
}

/// Bias wrapper: clamp `Refuse` and `Escalate` to `Ask` — dev pack actions
/// must always surface for human review, never auto-merge or auto-block.
#[must_use]
pub fn select_instinct(
    snap: &CompiledFieldSnapshot,
    posture: &PostureBundle,
    ctx: &ContextBundle,
) -> AutonomicInstinct {
    let base = crate::instinct::select_instinct_v0(snap, posture, ctx);
    match base {
        AutonomicInstinct::Refuse | AutonomicInstinct::Escalate => AutonomicInstinct::Ask,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBit, PostureBit};

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
        assert_eq!(select_instinct(&snap, &posture, &ctx), AutonomicInstinct::Ask);
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
        assert_eq!(select_instinct(&snap, &posture, &ctx), AutonomicInstinct::Ask);
    }

    #[test]
    fn settle_passes_through() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::SETTLED,
            confidence: 200,
        };
        assert_eq!(
            select_instinct(&snap, &posture, &ContextBundle::default()),
            AutonomicInstinct::Settle
        );
    }
}
