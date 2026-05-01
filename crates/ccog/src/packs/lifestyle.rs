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
use crate::construct8::{Construct8, ObjectId, PredicateId, Triple};
use crate::ids::{BreedId, EdgeId, FieldId, GroupId, NodeId, PackId, RuleId};
use crate::instinct::AutonomicInstinct;
use crate::packs::bits::LIFESTYLE_RANGE;
use crate::packs::FieldPack;
use crate::runtime::cog8::{
    Cog8Edge, Cog8Row, CollapseFn, EdgeKind, Instinct, Powl8Instr, Powl8Op,
};
use crate::runtime::ClosedFieldContext;
use crate::utils::dense::fnv1a_64;
use crate::verdict::Breed;
use anyhow::Result;

/// Lifestyle / OT pack numeric ID.
pub const PACK_ID: PackId = PackId(1);

/// Lifestyle pack posture bits — local within the [`LIFESTYLE_RANGE`] band.
#[allow(non_snake_case)]
pub mod Bit {
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

/// Alias for [`Bit`] to match standard naming conventions.
pub use Bit as LifestyleBit;

ccog_const_assert!(Bit::ROUTINE_DUE >= LIFESTYLE_RANGE.start);
ccog_const_assert!(Bit::AFFORDANCE_OFFERED < LIFESTYLE_RANGE.end);

/// COG8 rows for the Lifestyle pack.
pub static COG8_NODES: &[Cog8Row] = &[
    // Node 0: Start marker (Silent)
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
    // Node 1: routine_due_check
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(Breed::Mycin as u8),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [
            FieldId(Bit::ROUTINE_DUE as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::ROUTINE_DUE,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ask,
        priority: 10,
    },
    // Node 2: fatigue_acknowledge
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(2),
        rule_id: RuleId(2),
        breed_id: BreedId(Breed::Eliza as u8),
        collapse_fn: CollapseFn::ReflectivePosture,
        var_ids: [
            FieldId(Bit::FATIGUED as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::FATIGUED,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 20,
    },
    // Node 3: transition_smooth
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(3),
        rule_id: RuleId(3),
        breed_id: BreedId(Breed::Strips as u8),
        collapse_fn: CollapseFn::Preconditions,
        var_ids: [
            FieldId(Bit::TRANSITION_OPEN as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::TRANSITION_OPEN,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 30,
    },
    // Node 4: affordance_offer
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(4),
        rule_id: RuleId(4),
        breed_id: BreedId(Breed::Shrdlu as u8),
        collapse_fn: CollapseFn::Grounding,
        var_ids: [
            FieldId(Bit::AFFORDANCE_OFFERED as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::AFFORDANCE_OFFERED,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Inspect,
        priority: 40,
    },
    // Node 5: overstim_settle
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(5),
        rule_id: RuleId(5),
        breed_id: BreedId(Breed::Eliza as u8),
        collapse_fn: CollapseFn::ReflectivePosture,
        var_ids: [
            FieldId(Bit::OVERSTIMULATED as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::OVERSTIMULATED,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 50,
    },
];

/// POWL8 topology edges for the Lifestyle pack.
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
            guard_mask: 1u64 << 0,
            effect_mask: 1u64 << 1,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(2),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ReflectivePosture,
            node_id: NodeId(2),
            edge_id: EdgeId(2),
            guard_mask: 1u64 << 0,
            effect_mask: 1u64 << 2,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(3),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::Preconditions,
            node_id: NodeId(3),
            edge_id: EdgeId(3),
            guard_mask: 1u64 << 0,
            effect_mask: 1u64 << 3,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(4),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::Grounding,
            node_id: NodeId(4),
            edge_id: EdgeId(4),
            guard_mask: 1u64 << 0,
            effect_mask: 1u64 << 4,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(5),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ReflectivePosture,
            node_id: NodeId(5),
            edge_id: EdgeId(5),
            guard_mask: 1u64 << 0,
            effect_mask: 1u64 << 5,
        },
    },
];

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
    let activity_uri = format!("urn:blake3:{}", h.to_hex());
    let rt_uri = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let prov_activity_uri = "http://www.w3.org/ns/prov#Activity";

    let subject = ObjectId(fnv1a_64(activity_uri.as_bytes()) as u32);
    let predicate = PredicateId(fnv1a_64(rt_uri.as_bytes()) as u16);
    let object = ObjectId(fnv1a_64(prov_activity_uri.as_bytes()) as u32);

    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(subject, predicate, object));
    Ok(delta)
}

fn act_routine_due_check(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"lifestyle/routine_due_check")
}

fn act_fatigue_acknowledge(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"lifestyle/fatigue_acknowledge")
}

fn act_transition_smooth(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"lifestyle/transition_smooth")
}

fn act_affordance_offer(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"lifestyle/affordance_offer")
}

fn act_overstim_settle(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"lifestyle/overstim_settle")
}

/// Bias wrapper: when the subject is fatigued or overstimulated, refusal is
/// not cooperative; rewrite `Refuse` to `Ask`. All other classes pass through
/// unchanged. Never introduces new variants.
#[must_use]
pub fn select_instinct(context: &ClosedFieldContext) -> AutonomicInstinct {
    let base = crate::instinct::select_instinct_v0(context);
    let fatigued = context.posture.has(Bit::FATIGUED) || context.posture.has(Bit::OVERSTIMULATED);
    if fatigued && matches!(base, AutonomicInstinct::Refuse) {
        return AutonomicInstinct::Ask;
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBundle, PostureBit, PostureBundle};
    use crate::packs::TierMasks;

    fn empty_snap() -> CompiledFieldSnapshot {
        let f = FieldContext::new("t");
        CompiledFieldSnapshot::from_field(&f).expect("snap")
    }

    #[test]
    fn fatigued_rewrites_refuse_to_ask() {
        let snap = empty_snap();
        let posture = PostureBundle {
            posture_mask: (1u64 << PostureBit::ALERT) | (1u64 << Bit::FATIGUED),
            confidence: 200,
        };
        let ctx = ContextBundle {
            expectation_mask: 0,
            risk_mask: 1u64 << crate::multimodal::ContextBit::THEFT_RISK,
            affordance_mask: 0,
        };
        let context = ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx,
            tiers: TierMasks::ZERO,
        };
        // Base lattice would be Refuse; Lifestyle bias turns it into Ask.
        assert_eq!(select_instinct(&context), AutonomicInstinct::Ask);
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
        let context = ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture,
            context: ctx,
            tiers: TierMasks::ZERO,
        };
        assert_eq!(select_instinct(&context), AutonomicInstinct::Refuse);
    }

    #[test]
    fn all_acts_emit_one_triple() {
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
            assert_eq!(
                delta.len(),
                1,
                "lifestyle act {} must emit exactly one triple",
                slot.name
            );
        }
    }
}
