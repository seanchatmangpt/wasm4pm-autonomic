//! Edge / Home field pack.
//!
//! Targets home-edge cognition: package retrieval, visitor inspection, theft
//! escalation, settle-after-acknowledge. Emits only `urn:blake3:` IRIs for
//! any subject that could otherwise carry PII (visitor identity, address,
//! token). Admits STRIPS, SHRDLU, ELIZA — transition admissibility, object
//! affordance, and phrase binding cover the home-edge surface.

use crate::bark_artifact::BarkSlot;
use crate::ccog_const_assert;
use crate::compiled::CompiledFieldSnapshot;
use crate::construct8::Construct8;
use crate::instinct::AutonomicInstinct;
use crate::multimodal::{ContextBundle, PostureBundle};
use crate::packs::bits::EDGE_RANGE;
use crate::packs::FieldPack;
use crate::verdict::Breed;
use anyhow::Result;
use oxigraph::model::{NamedNode, Term, Triple};

/// Edge pack posture/context bits — local within the [`EDGE_RANGE`] band.
#[allow(non_snake_case)]
pub mod EdgeBit {
    /// A package has arrived at the edge.
    pub const PACKAGE_AT_EDGE: u32 = 32;
    /// An unknown visitor is present.
    pub const VISITOR_PRESENT: u32 = 33;
    /// The acknowledgement signal has fired.
    pub const ACK_SIGNAL: u32 = 34;
    /// Theft pattern detected (e.g. lingering, repeated approach).
    pub const THEFT_PATTERN: u32 = 35;
}

ccog_const_assert!(EdgeBit::PACKAGE_AT_EDGE >= EDGE_RANGE.start);
ccog_const_assert!(EdgeBit::THEFT_PATTERN < EDGE_RANGE.end);

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
    let activity = NamedNode::new(format!("urn:blake3:{}", h.to_hex()))?;
    let rt = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
    let prov_activity = NamedNode::new("http://www.w3.org/ns/prov#Activity")?;
    let act_term: Term = prov_activity.into();
    let mut delta = Construct8::empty();
    let _ = delta.push(Triple::new(activity, rt, act_term));
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

fn act_package_retrieve(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"edge/package_retrieve")
}

fn act_visitor_inspect(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"edge/visitor_inspect")
}

fn act_theft_escalate(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"edge/theft_escalate")
}

fn act_settle_after_ack(_snap: &CompiledFieldSnapshot) -> Result<Construct8> {
    pack_activity(b"edge/settle_after_ack")
}

/// Bias wrapper: edge pack is purely additive — pass the canonical lattice
/// through unmodified. Never introduces new variants.
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

    #[test]
    fn pii_marker_detector_flags_at_sign() {
        assert!(contains_pii_marker(b"alice@example.com"));
        assert!(!contains_pii_marker(b"edge/package_retrieve"));
    }

    #[test]
    fn all_acts_emit_only_blake3_urns() {
        use crate::field::FieldContext;
        let f = FieldContext::new("t");
        let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
        for slot in BUILTINS {
            let delta = (slot.act)(&snap).expect("act");
            for triple in delta.iter() {
                let s = triple.subject.to_string();
                assert!(
                    s.contains("urn:blake3:"),
                    "edge subject must be urn:blake3, got {s}"
                );
            }
        }
    }
}
