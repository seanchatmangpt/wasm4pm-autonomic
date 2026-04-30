//! Lifestyle / OT field pack.
//!
//! Targets routine-cadence cognition: routine due / fatigue / transition /
//! affordance offer / overstim settle. Bias: when the posture surface
//! signals fatigue or overstimulation, rewrite a `Refuse` decision into an
//! `Ask` — refusal in a depleted state is non-cooperative; ask instead.
//!
//! Admits ELIZA, MYCIN, Prolog (phrase binding, evidence gap, transitive
//! relation) — all of which are appropriate for routine OT contexts.

use crate::bark_artifact::BarkSlot;
use crate::ccog_const_assert;
use crate::compiled::CompiledFieldSnapshot;
use crate::construct8::Construct8;
use crate::instinct::AutonomicInstinct;
use crate::multimodal::{ContextBundle, PostureBundle};
use crate::packs::bits::LIFESTYLE_RANGE;
use crate::packs::FieldPack;
use crate::verdict::Breed;
use anyhow::Result;
use oxigraph::model::{NamedNode, Term, Triple};

/// Lifestyle pack posture bits — local within the [`LIFESTYLE_RANGE`] band.
#[allow(non_snake_case)]
pub mod LifestyleBit {
    /// Routine cadence is currently due.
    pub const ROUTINE_DUE: u32 = 16;
    /// Subject is fatigued.
    pub const FATIGUED: u32 = 17;
    /// Subject is overstimulated.
    pub const OVERSTIMULATED: u32 = 18;
    /// A transition window is open (e.g. shift-change, sleep, wake).
    pub const TRANSITION_OPEN: u32 = 19;
    /// An affordance is currently offered.
    pub const AFFORDANCE_OFFERED: u32 = 20;
}

ccog_const_assert!(LifestyleBit::ROUTINE_DUE >= LIFESTYLE_RANGE.start);
ccog_const_assert!(LifestyleBit::AFFORDANCE_OFFERED < LIFESTYLE_RANGE.end);

/// Lifestyle / OT pack handle (zero-sized).
pub struct LifestylePack;

impl FieldPack for LifestylePack {
    const NAME: &'static str = "lifestyle";
    const ONTOLOGY_PROFILE: &'static [&'static str] = &[
        "http://www.w3.org/ns/prov#",
        "https://schema.org/",
        "urn:blake3:",
        "urn:ccog:vocab:",
    ];
    const ADMITTED_BREEDS: &'static [Breed] = &[Breed::Eliza, Breed::Mycin, Breed::Prolog];
    const POSTURE_RANGE: core::ops::Range<u32> = LIFESTYLE_RANGE;
    const CONTEXT_RANGE: core::ops::Range<u32> = LIFESTYLE_RANGE;

    fn builtins() -> &'static [BarkSlot] {
        BUILTINS
    }
}

/// Static const table of Lifestyle pack bark slots.
pub static BUILTINS: &[BarkSlot] = &[
    BarkSlot {
        name: "routine_due_check",
        require_mask: 0,
        act: act_routine_due_check,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "fatigue_acknowledge",
        require_mask: 0,
        act: act_fatigue_acknowledge,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "transition_smooth",
        require_mask: 0,
        act: act_transition_smooth,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "affordance_offer",
        require_mask: 0,
        act: act_affordance_offer,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "overstim_settle",
        require_mask: 0,
        act: act_overstim_settle,
        emit_receipt: true,
        predecessor_mask: 0,
    },
];

fn pack_activity(tag: &[u8]) -> Result<Construct8> {
    let h = blake3::hash(tag);
    let activity = NamedNode::new(format!("urn:blake3:{}", h.to_hex()))?;
    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let prov_activity = NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let act_term: Term = prov_activity.into();
    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(activity, rt, act_term));
    Ok(delta)
}

fn act_routine_due_check(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"lifestyle/routine_due_check")
}

fn act_fatigue_acknowledge(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"lifestyle/fatigue_acknowledge")
}

fn act_transition_smooth(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"lifestyle/transition_smooth")
}

fn act_affordance_offer(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"lifestyle/affordance_offer")
}

fn act_overstim_settle(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"lifestyle/overstim_settle")
}

/// Bias wrapper: when the subject is fatigued or overstimulated, refusal is
/// not cooperative; rewrite `Refuse` to `Ask`. All other classes pass through
/// unchanged. Never introduces new variants.
#[must_use]
pub fn select_instinct(
    snap: &CompiledFieldSnapshot,
    posture: &PostureBundle,
    ctx: &ContextBundle,
) -> AutonomicInstinct {
    let base = crate::instinct::select_instinct_v0(snap, posture, ctx);
    let fatigued = posture.has(LifestyleBit::FATIGUED) || posture.has(LifestyleBit::OVERSTIMULATED);
    if fatigued && matches!(base, AutonomicInstinct::Refuse) {
        return AutonomicInstinct::Ask;
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldContext;
    use crate::multimodal::PostureBit;

    fn empty_snap() -> CompiledFieldSnapshot {
        let f = FieldContext::new("t");
        CompiledFieldSnapshot::from_field(&f).expect("snap")
    }

    #[test]
    fn fatigued_rewrites_refuse_to_ask() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: (1u64 << PostureBit::ALERT) | (1u64 << LifestyleBit::FATIGUED),
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << crate::multimodal::ContextBit::THEFT_RISK,
            affordance_mask: 0,
        };
        // Base lattice would be Refuse; Lifestyle bias turns it into Ask.
        assert_eq!(select_instinct(&snap, &posture, &ctx), AutonomicInstinct::Ask);
    }

    #[test]
    fn unfatigued_passes_through() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: 1u64 << PostureBit::ALERT,
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << crate::multimodal::ContextBit::THEFT_RISK,
            affordance_mask: 0,
        };
        assert_eq!(select_instinct(&snap, &posture, &ctx), AutonomicInstinct::Refuse);
    }
}
