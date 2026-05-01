//! JSON-LD serializer for Phase 11 audit surface.
#![allow(clippy::disallowed_types)]

use std::collections::BTreeMap;

use serde_json::Value;

use crate::receipt::Receipt;
use crate::trace::{BarkSkipReason, CcogTrace};

/// Static JSON-LD `@context` shared by every Phase 11 export.
pub const JSONLD_CONTEXT: &[(&str, &str)] = &[
    ("prov", "http://www.w3.org/ns/prov#"),
    ("schema", "https://schema.org/"),
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("sh", "http://www.w3.org/ns/shacl#"),
    ("ccog", "urn:ccog:vocab:"),
];

fn context_value() -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in JSONLD_CONTEXT {
        m.insert((*k).to_string(), Value::String((*v).to_string()));
    }
    btreemap_to_value(m)
}

fn btreemap_to_value(m: BTreeMap<String, Value>) -> Value {
    let mut out = serde_json::Map::new();
    for (k, v) in m {
        out.insert(k, v);
    }
    Value::Object(out)
}

/// Serialize `trace` as a JSON-LD value with PROV-shaped nodes.
pub fn trace_to_jsonld(trace: &CcogTrace) -> Value {
    let mut root: BTreeMap<String, Value> = BTreeMap::new();
    root.insert("@context".into(), context_value());
    root.insert(
        "@type".into(),
        Value::String("urn:ccog:vocab:CcogTrace".into()),
    );
    root.insert(
        "urn:ccog:vocab:presentMask".into(),
        Value::String(format!("0x{:016x}", trace.present_mask)),
    );
    root.insert(
        "urn:ccog:vocab:posture".into(),
        Value::String(format!("urn:ccog:vocab:posture/{:?}", trace.posture)),
    );

    let mut nodes: Vec<Value> = Vec::with_capacity(trace.nodes.len());
    for n in &trace.nodes {
        let mut node: BTreeMap<String, Value> = BTreeMap::new();
        node.insert(
            "@type".into(),
            Value::String("urn:ccog:vocab:BarkNodeTrace".into()),
        );
        node.insert(
            "urn:ccog:vocab:slotIdx".into(),
            Value::String(n.slot_idx.to_string()),
        );
        node.insert(
            "urn:ccog:vocab:hookId".into(),
            Value::String(n.hook_id.to_string()),
        );
        node.insert(
            "urn:ccog:vocab:requireMask".into(),
            Value::String(format!("0x{:016x}", n.require_mask)),
        );
        node.insert(
            "urn:ccog:vocab:predecessorMask".into(),
            Value::String(format!("0x{:016x}", n.predecessor_mask)),
        );
        node.insert(
            "urn:ccog:vocab:triggerFired".into(),
            Value::Bool(n.trigger_fired),
        );
        node.insert(
            "urn:ccog:vocab:checkPassed".into(),
            Value::Bool(n.check_passed),
        );
        node.insert(
            "urn:ccog:vocab:actEmittedTriples".into(),
            Value::String(n.act_emitted_triples.to_string()),
        );
        if let Some(skip) = n.skip {
            node.insert(
                "urn:ccog:vocab:skipReason".into(),
                Value::String(skip_reason_token(skip).into()),
            );
        }
        if let Some(urn) = &n.receipt_urn {
            node.insert(
                "urn:ccog:vocab:receiptUrn".into(),
                Value::String(urn.clone()),
            );
        }
        nodes.push(btreemap_to_value(node));
    }
    root.insert("urn:ccog:vocab:nodes".into(), Value::Array(nodes));
    btreemap_to_value(root)
}

fn skip_reason_token(s: BarkSkipReason) -> &'static str {
    match s {
        BarkSkipReason::PredecessorNotAdvanced => "PredecessorNotAdvanced",
        BarkSkipReason::RequireMaskUnsatisfied => "RequireMaskUnsatisfied",
        BarkSkipReason::NoSlot => "NoSlot",
        BarkSkipReason::ManualOnly => "ManualOnly",
        BarkSkipReason::CheckFailed => "CheckFailed",
        BarkSkipReason::ActNotMaterialized => "ActNotMaterialized",
        BarkSkipReason::ReceiptDisabled => "ReceiptDisabled",
    }
}

/// Serialize `receipt` as a `prov:Activity` JSON-LD value.
pub fn receipt_to_jsonld(receipt: &Receipt) -> Value {
    let mut m: BTreeMap<String, Value> = BTreeMap::new();
    m.insert("@context".into(), context_value());
    m.insert(
        "@id".into(),
        Value::String(receipt.activity_iri.as_str().to_string()),
    );
    m.insert(
        "@type".into(),
        Value::String("http://www.w3.org/ns/prov#Activity".into()),
    );
    m.insert(
        "urn:ccog:vocab:hash".into(),
        Value::String(receipt.hash.clone()),
    );
    m.insert(
        "urn:ccog:vocab:generatedAtTime".into(),
        Value::String(receipt.timestamp.to_rfc3339()),
    );
    btreemap_to_value(m)
}

/// Canonical byte form of a JSON-LD value.
pub fn canonical_bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).expect("Value is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBundle, PostureBundle};
    use crate::packs::TierMasks;
    use crate::runtime::ClosedFieldContext;
    use crate::trace::trace_default_builtins;

    #[test]
    fn jsonld_trace_roundtrip_stable_bytes() {
        let mut field = FieldContext::new("test");
        field
            .load_field_state(
                "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
            )
            .expect("load");
        let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };
        let trace = trace_default_builtins(&context);
        let v1 = trace_to_jsonld(&trace);
        let v2 = trace_to_jsonld(&trace);
        assert_eq!(canonical_bytes(&v1), canonical_bytes(&v2));
    }

    #[test]
    fn jsonld_context_only_public_iris() {
        for (_, iri) in JSONLD_CONTEXT {
            let v = serde_json::json!({"@id": iri});
            super::super::ontology::audit_iris(&v, &[])
                .unwrap_or_else(|e| panic!("context IRI must be public: {} ({})", iri, e));
        }
    }
}
