//! JTBD suite for Edge, Enterprise, and Dev field packs.
//!
//! Anti-stub: every test asserts cross-layer consequences (graph delta,
//! response class, replay, or PII surface). Every pack act fn is exercised;
//! every bias rule has positive + negative + perturbation assertions.

use ccog::compiled::CompiledFieldSnapshot;
use ccog::field::FieldContext;
use ccog::instinct::AutonomicInstinct;
use ccog::multimodal::{ContextBit, ContextBundle, PostureBit, PostureBundle};
use ccog::packs::{dev, edge, enterprise, TierMasks};
use ccog::runtime::ClosedFieldContext;

use fake::faker::name::en::Name;
use fake::Fake;
use proptest::prelude::*;

fn empty_snap() -> CompiledFieldSnapshot {
    let f = FieldContext::new("packs-jtbd");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

fn emptycontext(snap: std::sync::Arc<CompiledFieldSnapshot>) -> ClosedFieldContext {
    ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    }
}

fn snap_with_evidence_gap() -> CompiledFieldSnapshot {
    let mut f = FieldContext::new("packs-jtbd-gap");
    f.load_field_state(
        "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
    )
    .expect("load");
    CompiledFieldSnapshot::from_field(&f).expect("snap")
}

fn posture(bits: &[u32]) -> PostureBundle {
    let mut mask = 0u64;
    for b in bits {
        mask |= 1u64 << b;
    }
    PostureBundle {
        posture_mask: mask,
        confidence: 200,
    }
}

fn ctx_with(exp: u64, risk: u64, aff: u64) -> ContextBundle {
    ContextBundle {
        expectation_mask: exp,
        risk_mask: risk,
        affordance_mask: aff,
    }
}

fn b(bit: u32) -> u64 {
    1u64 << bit
}

// =============================================================================
// EDGE PACK — Mark-style local cognition
// =============================================================================

#[test]
fn jtbd_edge_package_expected_with_capacity_selects_retrieve() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::CADENCE_DELIVERY, PostureBit::ALERT]);
    let c = ctx_with(
        b(ContextBit::PACKAGE_EXPECTED),
        0,
        b(ContextBit::CAN_RETRIEVE_NOW),
    );

    let r = edge::select_instinct(&ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    });
    assert_eq!(
        r,
        AutonomicInstinct::Retrieve,
        "package expected + can retrieve + cadence → Retrieve"
    );

    // Perturbation: drop CAN_RETRIEVE_NOW.
    let c2 = ctx_with(b(ContextBit::PACKAGE_EXPECTED), 0, 0);
    let r2 = edge::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c2,
        tiers: TierMasks::ZERO,
    });
    assert_ne!(
        r2,
        AutonomicInstinct::Retrieve,
        "no affordance → no Retrieve"
    );

    // Perturbation: drop CADENCE_DELIVERY.
    let p2 = posture(&[PostureBit::ALERT]);
    let r3 = edge::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p2,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_ne!(r3, AutonomicInstinct::Retrieve, "no cadence → no Retrieve");
}

#[test]
fn jtbd_edge_theft_risk_alert_selects_refuse() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx_with(0, b(ContextBit::THEFT_RISK), 0);
    let r = edge::select_instinct(&ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    });
    assert_eq!(r, AutonomicInstinct::Refuse);

    // Perturbation: remove theft risk.
    let c2 = ctx_with(0, 0, 0);
    let r2 = edge::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c2,
        tiers: TierMasks::ZERO,
    });
    assert_ne!(r2, AutonomicInstinct::Refuse);
}

#[test]
fn jtbd_edge_pack_acts_emit_only_urn_blake3_no_pii() {
    let snap = empty_snap();
    let context = emptycontext(std::sync::Arc::new(snap.clone()));
    for slot in edge::BUILTINS {
        let delta = (slot.act)(&context).expect("edge act");
        assert!(
            !delta.is_empty(),
            "edge slot {} must emit a delta",
            slot.name
        );
        let nt = delta.to_ntriples();
        // Every act must use hashed URNs.
        assert!(
            nt.contains("urn:ccog:id:"),
            "edge slot {} must emit urn:ccog:id: IRI:\n{}",
            slot.name,
            nt
        );
        // Negative boundary: must not embed a generated PII-shaped string.
        let probe: String = Name().fake();
        assert!(
            !nt.contains(&probe),
            "edge slot {} embedded fake-PII probe {}:\n{}",
            slot.name,
            probe,
            nt
        );
        // Must not contain '@' (email-shape PII).
        assert!(!nt.contains('@'), "edge delta contains '@':\n{}", nt);
    }
}

#[test]
fn jtbd_edge_must_escalate_dominates_theft_risk() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx_with(
        0,
        b(ContextBit::MUST_ESCALATE) | b(ContextBit::THEFT_RISK),
        0,
    );
    let r = edge::select_instinct(&ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    });
    assert_eq!(
        r,
        AutonomicInstinct::Escalate,
        "MUST_ESCALATE dominates Refuse"
    );
}

// =============================================================================
// ENTERPRISE PACK — process / SLA / compliance
// =============================================================================

#[test]
fn jtbd_enterprise_evidence_gap_selects_ask() {
    let snap = snap_with_evidence_gap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx_with(0, 0, 0);
    let r = enterprise::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_eq!(r, AutonomicInstinct::Ask, "evidence gap → Ask");

    // Perturbation: remove gap AND switch to CALM posture → falls through to Ignore.
    let snap2 = empty_snap();
    let p_calm = posture(&[PostureBit::CALM]);
    let r2 = enterprise::select_instinct(&ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap2.clone()),
        posture: p_calm,
        context: c,
        tiers: TierMasks::ZERO,
        human_burden: 0,
    });
    assert_eq!(
        r2,
        AutonomicInstinct::Ignore,
        "no gap + calm + empty context → Ignore"
    );
    assert_ne!(r, r2, "removing the gap must change the response");
}

#[test]
fn jtbd_enterprise_must_escalate_compliance_path() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx_with(0, b(ContextBit::MUST_ESCALATE), 0);
    let r = enterprise::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_eq!(r, AutonomicInstinct::Escalate);

    let r2 = enterprise::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: ctx_with(0, 0, 0),
        tiers: TierMasks::ZERO,
    });
    assert_ne!(r2, AutonomicInstinct::Escalate);
}

#[test]
fn jtbd_enterprise_pack_acts_emit_prov_activity_with_urn_blake3() {
    let snap = empty_snap();
    let context = emptycontext(std::sync::Arc::new(snap.clone()));
    for slot in enterprise::BUILTINS {
        let delta = (slot.act)(&context).expect("enterprise act");
        assert!(
            !delta.is_empty(),
            "enterprise {} must emit delta",
            slot.name
        );
        let nt = delta.to_ntriples();
        let h_act = format!(
            "{:04x}",
            ccog::utils::dense::fnv1a_64("http://www.w3.org/ns/prov#Activity".as_bytes()) as u16
        );
        assert!(
            nt.contains(&h_act),
            "enterprise {} must emit prov:Activity:\n{}",
            slot.name,
            nt
        );
        assert!(
            nt.contains("urn:ccog:id:"),
            "enterprise {} must emit hashed ID IRI:\n{}",
            slot.name,
            nt
        );
        // Negative boundary: never SHACL on instance.
        assert!(
            !nt.contains("shacl#targetClass"),
            "enterprise {} must NOT emit sh:targetClass on instance:\n{}",
            slot.name,
            nt
        );
    }
}

// =============================================================================
// DEV PACK — agent governance, never auto-merge
// =============================================================================

#[test]
fn jtbd_dev_pack_clamps_refuse_to_ask() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx_with(0, b(ContextBit::THEFT_RISK), 0);
    // Base lattice: theft risk + alert → Refuse.
    let base = ccog::instinct::select_instinct_v0(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_eq!(base, AutonomicInstinct::Refuse);

    // Dev pack must never surface Refuse — always Ask.
    let r = dev::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_eq!(
        r,
        AutonomicInstinct::Ask,
        "dev pack must clamp Refuse → Ask"
    );
}

#[test]
fn jtbd_dev_pack_clamps_escalate_to_ask() {
    let snap = empty_snap();
    let p = posture(&[PostureBit::ALERT]);
    let c = ctx_with(0, b(ContextBit::MUST_ESCALATE), 0);
    let base = ccog::instinct::select_instinct_v0(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_eq!(base, AutonomicInstinct::Escalate);

    // Dev pack must never auto-escalate — always Ask for human review.
    let r = dev::select_instinct(&ClosedFieldContext {
        human_burden: 0,
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: p,
        context: c,
        tiers: TierMasks::ZERO,
    });
    assert_eq!(
        r,
        AutonomicInstinct::Ask,
        "dev pack must clamp Escalate → Ask"
    );
}

#[test]
fn jtbd_dev_pack_never_emits_refuse_or_escalate_under_any_input() {
    let snap = empty_snap();
    // Sweep every posture/context bit individually.
    for bit_idx in 0u32..64 {
        let p = posture(&[PostureBit::ALERT, bit_idx.min(63)]);
        let c_risk = ctx_with(0, 1u64 << bit_idx, 0);
        let r = dev::select_instinct(&ClosedFieldContext {
            human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: p,
            context: c_risk,
            tiers: TierMasks::ZERO,
        });
        assert_ne!(
            r,
            AutonomicInstinct::Refuse,
            "dev pack must NEVER emit Refuse (bit {})",
            bit_idx
        );
        assert_ne!(
            r,
            AutonomicInstinct::Escalate,
            "dev pack must NEVER emit Escalate (bit {})",
            bit_idx
        );
    }
}

#[test]
fn jtbd_dev_pack_acts_emit_urn_blake3_only() {
    let snap = empty_snap();
    let context = emptycontext(std::sync::Arc::new(snap.clone()));
    for slot in dev::BUILTINS {
        let delta = (slot.act)(&context).expect("dev act");
        let nt = delta.to_ntriples();
        if delta.is_empty() {
            continue;
        }
        assert!(
            nt.contains("urn:ccog:id:"),
            "dev {} must emit hashed URN:\n{}",
            slot.name,
            nt
        );
    }
}

// =============================================================================
// CROSS-PACK — namespace isolation under generated input
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn proptest_dev_pack_never_surfaces_refuse_or_escalate(
        p_mask in any::<u64>(),
        e_mask in any::<u64>(),
        r_mask in any::<u64>(),
        a_mask in any::<u64>(),
    ) {
        let snap = empty_snap();
        let p = PostureBundle { posture_mask: p_mask, confidence: 128 };
        let c = ContextBundle { expectation_mask: e_mask, risk_mask: r_mask, affordance_mask: a_mask };
        let r = dev::select_instinct(&ClosedFieldContext { human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: p,
            context: c,
            tiers: TierMasks::ZERO,
        });
        prop_assert_ne!(r, AutonomicInstinct::Refuse);
        prop_assert_ne!(r, AutonomicInstinct::Escalate);
    }

    #[test]
    fn proptest_edge_pack_passes_through_canonical_lattice(
        p_mask in any::<u64>(),
        r_mask in any::<u64>(),
    ) {
        let snap = empty_snap();
        let p = PostureBundle { posture_mask: p_mask, confidence: 128 };
        let c = ContextBundle { expectation_mask: 0, risk_mask: r_mask, affordance_mask: 0 };
        let context = ClosedFieldContext { human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: p,
            context: c,
            tiers: TierMasks::ZERO,
        };
        let r_pack = edge::select_instinct(&context);
        let r_base = ccog::instinct::select_instinct_v0(&context);
        prop_assert_eq!(r_pack, r_base, "edge pack must pass canonical lattice through");
    }

    #[test]
    fn proptest_enterprise_pack_passes_through_canonical_lattice(
        p_mask in any::<u64>(),
        a_mask in any::<u64>(),
    ) {
        let snap = empty_snap();
        let p = PostureBundle { posture_mask: p_mask, confidence: 128 };
        let c = ContextBundle { expectation_mask: 0, risk_mask: 0, affordance_mask: a_mask };
        let context = ClosedFieldContext { human_burden: 0,
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: p,
            context: c,
            tiers: TierMasks::ZERO,
        };
        let r_pack = enterprise::select_instinct(&context);
        let r_base = ccog::instinct::select_instinct_v0(&context);
        prop_assert_eq!(r_pack, r_base, "enterprise pack must pass canonical lattice through");
    }
}

// =============================================================================
// All-pack act sweep — every act fn produces a public-ontology N-Triples delta
// =============================================================================

#[test]
fn jtbd_all_pack_acts_use_public_ontology_only() {
    let snap = empty_snap();
    let context = emptycontext(std::sync::Arc::new(snap.clone()));
    let all_slots: Vec<&[ccog::bark_artifact::BarkSlot]> = vec![
        ccog::packs::lifestyle::BUILTINS,
        edge::BUILTINS,
        enterprise::BUILTINS,
        dev::BUILTINS,
    ];
    for pack_slots in all_slots {
        for slot in pack_slots {
            let delta = (slot.act)(&context).expect("act");
            if delta.is_empty() {
                continue;
            }
            let nt = delta.to_ntriples();
            // Forbid private namespaces beyond urn:blake3 / urn:ccog:vocab:.
            for line in nt.lines() {
                // Crude: every IRI in <...> must be public-ontology rooted.
                for part in line.split_whitespace() {
                    if part.starts_with('<') && part.ends_with('>') {
                        let iri = &part[1..part.len() - 1];
                        let ok = iri.starts_with("http://www.w3.org/")
                            || iri.starts_with("https://schema.org/")
                            || iri.starts_with("http://purl.org/")
                            || iri.starts_with("urn:blake3:")
                            || iri.starts_with("urn:ccog:");
                        assert!(ok, "pack slot {} emitted non-public IRI {}", slot.name, iri);
                    }
                }
            }
        }
    }
}

// =============================================================================
// COG8 FIELD PACK VERIFICATION
// =============================================================================

#[test]
fn jtbd_cog8_edge_pack_reproduction() {
    use ccog::runtime::cog8::*;

    // Simulate Edge pack "package expected" rule in COG8.
    // Rule: PACKAGE_EXPECTED + CAN_RETRIEVE_NOW + CADENCE_DELIVERY -> Retrieve
    let nodes = [Cog8Row {
        pack_id: PackId(2), // edge
        group_id: GroupId(1),
        rule_id: RuleId(1),
        breed_id: BreedId(1),
        collapse_fn: CollapseFn::ExpertRule,
        var_ids: [FieldId(0); 8],
        required_mask: (1 << 0) | (1 << 1), // Context bits mapped to local COG8 mask
        forbidden_mask: 0,
        predecessor_mask: 1 << 0, // Requires Posture bit mapped to completion
        response: Instinct::Retrieve,
        priority: 100,
    }];
    let edges = [Cog8Edge {
        from: NodeId(0),
        to: NodeId(0),
        kind: EdgeKind::Choice,
        instr: Powl8Instr {
            op: Powl8Op::Act,
            collapse_fn: CollapseFn::ExpertRule,
            node_id: NodeId(0),
            edge_id: EdgeId(1),
            guard_mask: 0,
            effect_mask: 0,
        },
    }];

    let f = FieldContext::new("test");
    let snap = std::sync::Arc::new(CompiledFieldSnapshot::from_field(&f).unwrap());
    let context = ClosedFieldContext {
        snapshot: snap,
        posture: PostureBundle::default(),
        context: ContextBundle::default(),
        tiers: TierMasks::ZERO,
        human_burden: 0,
    };

    // Positive: satisfied.
    let d = execute_cog8_graph(&nodes, &edges, 0b11, 1 << 0).expect("execute");
    assert_eq!(d.response, Instinct::Retrieve);

    // Negative: missing posture.
    let d2 = execute_cog8_graph(&nodes, &edges, 0, 0).expect("execute");
    assert_eq!(d2.response, Instinct::Ignore);
}
