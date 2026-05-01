//! Edge / Home field pack.
//!
//! Targets home-edge cognition: package retrieval, visitor inspection, theft
//! escalation, settle-after-acknowledge. Emits only `urn:blake3:` IRIs for
//! any subject that could otherwise carry PII (visitor identity, address,
//! token). Admits STRIPS, SHRDLU, ELIZA — transition admissibility, object
//! affordance, and phrase binding cover the home-edge surface.

use crate::bark_artifact::BarkSlot;
use crate::ccog_const_assert;
use crate::construct8::Construct8;
use crate::instinct::AutonomicInstinct;
use crate::packs::bits::EDGE_RANGE;
use crate::packs::FieldPack;
use crate::runtime::a2a::A2AProjectionTable;
use crate::runtime::cog8::{
    BreedId, Cog8Edge, Cog8Row, CollapseFn, EdgeId, EdgeKind, FieldId, GroupId, Instinct, NodeId,
    PackId, Powl8Instr, Powl8Op, RuleId,
};
use crate::runtime::mcp::{
    EffectPolicy, ExpectedResultType, MCPProjectionTable, ProjectionRule, ToolCallTemplate, ToolId,
};
use crate::runtime::ClosedFieldContext;
use crate::verdict::Breed;
use anyhow::Result;

/// Edge pack numeric ID.
pub const PACK_ID: PackId = PackId(2);

/// Edge pack posture/context bits — local within the [`EDGE_RANGE`] band.
#[allow(non_snake_case)]
pub mod Bit {
    /// A package has arrived at the edge.
    pub const PACKAGE_AT_EDGE: u32 = 32;
    /// An unknown visitor is present.
    pub const VISITOR_PRESENT: u32 = 33;
    /// The acknowledgement signal has fired.
    pub const ACK_SIGNAL: u32 = 34;
    /// Theft pattern detected (e.g. lingering, repeated approach).
    pub const THEFT_PATTERN: u32 = 35;
}

ccog_const_assert!(Bit::PACKAGE_AT_EDGE >= EDGE_RANGE.start);
ccog_const_assert!(Bit::THEFT_PATTERN < EDGE_RANGE.end);

/// COG8 nodes for the Edge pack.
pub static COG8_NODES: &[Cog8Row] = &[
    // Node 0: Root (Silent)
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(0),
        rule_id: RuleId(0),
        breed_id: BreedId(Breed::Strips as u8),
        collapse_fn: CollapseFn::None,
        var_ids: [FieldId(0); 8],
        required_mask: 0,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Ignore,
        priority: 0,
    },
    // Node 1: Package Retrieve (SHRDLU)
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(Breed::Shrdlu as u8),
        collapse_fn: CollapseFn::Grounding,
        var_ids: [
            FieldId(Bit::PACKAGE_AT_EDGE as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::PACKAGE_AT_EDGE,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Retrieve,
        priority: 10,
    },
    // Node 2: Visitor Inspect (ELIZA)
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(1),
        rule_id: RuleId(2),
        breed_id: BreedId(Breed::Eliza as u8),
        collapse_fn: CollapseFn::ReflectivePosture,
        var_ids: [
            FieldId(Bit::VISITOR_PRESENT as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::VISITOR_PRESENT,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Inspect,
        priority: 10,
    },
    // Node 3: Theft Escalate (SHRDLU)
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(1),
        rule_id: RuleId(3),
        breed_id: BreedId(Breed::Shrdlu as u8),
        collapse_fn: CollapseFn::Grounding,
        var_ids: [
            FieldId(Bit::THEFT_PATTERN as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::THEFT_PATTERN,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Escalate,
        priority: 20,
    },
    // Node 4: Settle After Ack (ELIZA)
    Cog8Row {
        pack_id: PACK_ID,
        group_id: GroupId(1),
        rule_id: RuleId(4),
        breed_id: BreedId(Breed::Eliza as u8),
        collapse_fn: CollapseFn::ReflectivePosture,
        var_ids: [
            FieldId(Bit::ACK_SIGNAL as u16),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
            FieldId(0),
        ],
        required_mask: 1u64 << Bit::ACK_SIGNAL,
        forbidden_mask: 0,
        predecessor_mask: 0,
        response: Instinct::Settle,
        priority: 5,
    },
];

/// POWL edges connecting COG8 nodes for the Edge pack.
pub static COG8_EDGES: &[Cog8Edge] = &[
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(1),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::Grounding,
            node_id: NodeId(1),
            edge_id: EdgeId(1),
            guard_mask: 1,
            effect_mask: 1 << 1,
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
            guard_mask: 1,
            effect_mask: 1 << 2,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(3),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::Grounding,
            node_id: NodeId(3),
            edge_id: EdgeId(3),
            guard_mask: 1,
            effect_mask: 1 << 3,
        },
    },
    Cog8Edge {
        from: NodeId(0),
        to: NodeId(4),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ReflectivePosture,
            node_id: NodeId(4),
            edge_id: EdgeId(4),
            guard_mask: 1,
            effect_mask: 1 << 4,
        },
    },
];

/// MCP Projection Table for Edge pack.
pub static MCP_PROJECTION_TABLE: MCPProjectionTable = MCPProjectionTable {
    rules: &[ProjectionRule {
        trigger_instinct: Instinct::Retrieve,
        collapse_fn: Some(CollapseFn::Grounding),
        template: ToolCallTemplate {
            tool_id: ToolId(1001), // get_package_status
            expected_result_type: ExpectedResultType::Construct8,
            effect_policy: EffectPolicy::Read,
            required_vars: 1u64 << Bit::PACKAGE_AT_EDGE,
        },
    }],
};

/// A2A Projection Table for Edge pack.
pub static A2A_PROJECTION_TABLE: A2AProjectionTable = A2AProjectionTable { rules: &[] };

/// Edge / Home pack handle (zero-sized).
pub struct EdgePack;

impl FieldPack for EdgePack {
    const NAME: &'static str = "edge";
    const ONTOLOGY_PROFILE: &'static [&'static str] = &[
        "http://www.w3.org/ns/prov#",
        "https://schema.org/",
        "urn:blake3:",
        "urn:ccog:vocab:",
    ];
    const ADMITTED_BREEDS: &'static [Breed] = &[Breed::Strips, Breed::Shrdlu, Breed::Eliza];
    const POSTURE_RANGE: core::ops::Range<u32> = EDGE_RANGE;
    const CONTEXT_RANGE: core::ops::Range<u32> = EDGE_RANGE;

    fn builtins() -> &'static [BarkSlot] {
        BUILTINS
    }
}

/// Static const table of Edge pack bark slots.
pub static BUILTINS: &[BarkSlot] = &[
    BarkSlot {
        name: "package_retrieve",
        require_mask: 0,
        act: act_package_retrieve,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "visitor_inspect",
        require_mask: 0,
        act: act_visitor_inspect,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "theft_escalate",
        require_mask: 0,
        act: act_theft_escalate,
        emit_receipt: true,
        predecessor_mask: 0,
    },
    BarkSlot {
        name: "settle_after_ack",
        require_mask: 0,
        act: act_settle_after_ack,
        emit_receipt: true,
        predecessor_mask: 0,
    },
];

/// PII guard: emit only blake3 URNs. Token is the static interpreter-issued
/// pack tag, never any visitor identifier.
fn pack_activity(tag: &[u8]) -> Result<Construct8> {
    debug_assert!(
        !contains_pii_marker(tag),
        "edge pack tag must not embed PII"
    );
    let h = blake3::hash(tag);
    let activity = format!("urn:blake3:{}", h.to_hex());
    let mut delta = Construct8::empty();
    let _ = delta.push(crate::construct8::Triple::from_strings(
        &activity,
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
        "http://www.w3.org/ns/prov#Activity",
    ));
    Ok(delta)
}

const fn contains_pii_marker(tag: &[u8]) -> bool {
    // Conservative scan: forbid '@' and ' ' which are common in raw PII tokens.
    let mut i = 0;
    while i < tag.len() {
        let b = tag[i];
        if b == b'@' || b == b' ' {
            return true;
        }
        i += 1;
    }
    false
}

fn act_package_retrieve(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"edge/package_retrieve")
}

fn act_visitor_inspect(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"edge/visitor_inspect")
}

fn act_theft_escalate(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"edge/theft_escalate")
}

fn act_settle_after_ack(_context: &ClosedFieldContext) -> Result<Construct8> {
    pack_activity(b"edge/settle_after_ack")
}

/// Bias wrapper: edge pack is purely additive — pass the canonical lattice
/// through unmodified. Never introduces new variants.
#[must_use]
pub fn select_instinct(context: &ClosedFieldContext) -> AutonomicInstinct {
    let decision = crate::runtime::cog8::execute_cog8(
        COG8_NODES, COG8_EDGES, context, 1, // Start with root node (Node 0) completed
    )
    .unwrap_or_default();

    if decision.response != Instinct::Ignore {
        return match decision.response {
            Instinct::Settle => AutonomicInstinct::Settle,
            Instinct::Retrieve => AutonomicInstinct::Retrieve,
            Instinct::Inspect => AutonomicInstinct::Inspect,
            Instinct::Ask => AutonomicInstinct::Ask,
            Instinct::Refuse => AutonomicInstinct::Refuse,
            Instinct::Escalate => AutonomicInstinct::Escalate,
            Instinct::Ignore => AutonomicInstinct::Ignore,
        };
    }

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
    fn pii_marker_detector_flags_at_sign() {
        assert!(contains_pii_marker(b"alice@example.com"));
        assert!(!contains_pii_marker(b"edge/package_retrieve"));
    }

    #[test]
    fn all_acts_emit_only_deterministic_ids() {
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
            assert!(
                !delta.is_empty(),
                "edge slot {} must emit a delta",
                slot.name
            );
            for triple in delta.iter() {
                // Ensure IDs are non-zero (hashed)
                assert_ne!(triple.subject.0, 0);
                assert_ne!(triple.predicate.0, 0);
                assert_ne!(triple.object.0, 0);
            }
        }
    }
}
