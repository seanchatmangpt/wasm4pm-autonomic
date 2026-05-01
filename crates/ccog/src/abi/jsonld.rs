//! JSON-LD serialization for POWL8 / POWL64 (Phase 10, offline only).
#![allow(clippy::disallowed_types)]
//!
//! Hand-rolled with `serde_json::Value` + `BTreeMap`-backed objects so the
//! emitted bytes are deterministic across runs and OS allocators.
//!
//! Vocabulary policy: ccog-internal terms use the `urn:ccog:vocab:` prefix
//! per the constitutional constraint — never bare `ccog:`. PROV-O,
//! schema.org, and SHACL terms are referenced by their public IRIs.
//!
//! Phase 11 ships a separate `export::jsonld` layer for receipt / trace
//! bundling; this module is the **POWL ABI specifically**.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::powl::{Powl8, Powl8Node};
use crate::powl64::Powl64;

fn context_value() -> Value {
    let mut ctx: Map<String, Value> = Map::new();
    ctx.insert(
        "prov".to_string(),
        Value::String("http://www.w3.org/ns/prov#".to_string()),
    );
    ctx.insert(
        "schema".to_string(),
        Value::String("https://schema.org/".to_string()),
    );
    ctx.insert(
        "sh".to_string(),
        Value::String("http://www.w3.org/ns/shacl#".to_string()),
    );
    ctx.insert(
        "xsd".to_string(),
        Value::String("http://www.w3.org/2001/XMLSchema#".to_string()),
    );
    ctx.insert(
        "ccog".to_string(),
        Value::String("urn:ccog:vocab:".to_string()),
    );
    Value::Object(ctx)
}

fn into_map(b: BTreeMap<&'static str, Value>) -> Map<String, Value> {
    let mut out: Map<String, Value> = Map::with_capacity(b.len());
    for (k, v) in b {
        out.insert(k.to_string(), v);
    }
    out
}

/// Render a [`Powl8`] as a deterministic JSON-LD value.
///
/// Layout:
///
/// ```jsonc
/// {
///   "@context": { ... },
///   "@type": "ccog:Powl8Plan",
///   "ccog:root": <u16>,
///   "ccog:nodes": [ <node>, ... ]
/// }
/// ```
///
/// **Offline only** — never on the hot path. The returned `Value` is a
/// `serde_json::Map<String, Value>`; serialization to bytes (e.g., via
/// `serde_json::to_string`) yields deterministic output because every
/// nested object is built from a `BTreeMap`.
pub fn powl8_to_jsonld(plan: &Powl8) -> Value {
    let mut root: BTreeMap<&'static str, Value> = BTreeMap::new();
    root.insert("@context", context_value());
    root.insert("@type", Value::String("ccog:Powl8Plan".to_string()));
    root.insert("ccog:root", Value::Number(plan.root.into()));
    let mut nodes: Vec<Value> = Vec::with_capacity(plan.nodes.len());
    for n in &plan.nodes {
        nodes.push(node_to_jsonld(*n));
    }
    root.insert("ccog:nodes", Value::Array(nodes));
    Value::Object(into_map(root))
}

fn node_to_jsonld(n: Powl8Node) -> Value {
    let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();
    match n {
        Powl8Node::Silent => {
            m.insert("@type", Value::String("ccog:Silent".to_string()));
        }
        Powl8Node::Activity(b) => {
            m.insert("@type", Value::String("ccog:Activity".to_string()));
            m.insert("ccog:breed", Value::Number((b as u8).into()));
        }
        Powl8Node::PartialOrder { start, count, rel } => {
            m.insert("@type", Value::String("ccog:PartialOrder".to_string()));
            m.insert("ccog:start", Value::Number(start.into()));
            m.insert("ccog:count", Value::Number(count.into()));
            // Encode each row as a decimal string to dodge JSON's 53-bit
            // integer precision cap on common parsers.
            let mut rows: Vec<Value> = Vec::with_capacity(64);
            for i in 0..64 {
                let mut row: u64 = 0;
                for j in 0..64 {
                    if rel.is_edge(i, j) {
                        row |= 1u64 << j;
                    }
                }
                rows.push(Value::String(row.to_string()));
            }
            m.insert("ccog:rel", Value::Array(rows));
        }
        Powl8Node::OperatorSequence { a, b } => {
            m.insert("@type", Value::String("ccog:OperatorSequence".to_string()));
            m.insert("ccog:a", Value::Number(a.into()));
            m.insert("ccog:b", Value::Number(b.into()));
        }
        Powl8Node::OperatorParallel { a, b } => {
            m.insert("@type", Value::String("ccog:OperatorParallel".to_string()));
            m.insert("ccog:a", Value::Number(a.into()));
            m.insert("ccog:b", Value::Number(b.into()));
        }
        Powl8Node::StartNode => {
            m.insert("@type", Value::String("ccog:StartNode".to_string()));
        }
        Powl8Node::EndNode => {
            m.insert("@type", Value::String("ccog:EndNode".to_string()));
        }
        Powl8Node::Choice { branches, len } => {
            m.insert("@type", Value::String("ccog:Choice".to_string()));
            let arr: Vec<Value> = branches
                .iter()
                .take(len as usize)
                .map(|b| Value::Number((*b).into()))
                .collect();
            m.insert("ccog:branches", Value::Array(arr));
            m.insert("ccog:len", Value::Number(len.into()));
        }
        Powl8Node::Loop { body, max_iters } => {
            m.insert("@type", Value::String("ccog:Loop".to_string()));
            m.insert("ccog:body", Value::Number(body.into()));
            m.insert("ccog:maxIters", Value::Number(max_iters.into()));
        }
    }
    Value::Object(into_map(m))
}

/// Render a [`Powl64`] chain-hash path as JSON-LD. Each chain entry is a
/// `urn:blake3:` IRI (BLAKE3 hex), preserving the canonical replay record.
///
/// **Offline only.**
pub fn powl64_to_jsonld(p: &Powl64) -> Value {
    let mut root: BTreeMap<&'static str, Value> = BTreeMap::new();
    root.insert("@context", context_value());
    root.insert("@type", Value::String("ccog:Powl64Path".to_string()));
    let arr: Vec<Value> = p
        .path()
        .iter()
        .map(|h| Value::String(format!("urn:blake3:{}", h.to_hex())))
        .collect();
    root.insert("ccog:chain", Value::Array(arr));
    Value::Object(into_map(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::powl::Powl8Node;
    use crate::verdict::Breed;

    #[test]
    fn powl8_jsonld_keys_are_deterministic() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::Activity(Breed::Gps)).unwrap();
        p.push(Powl8Node::EndNode).unwrap();
        let s1 = serde_json::to_string(&powl8_to_jsonld(&p)).unwrap();
        let s2 = serde_json::to_string(&powl8_to_jsonld(&p)).unwrap();
        assert_eq!(s1, s2);
        assert!(s1.contains("ccog:Powl8Plan"));
        assert!(s1.contains("\"ccog:nodes\""));
    }

    #[test]
    fn powl8_jsonld_emits_choice_and_loop_types() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::Activity(Breed::Eliza)).unwrap();
        p.push(Powl8Node::Activity(Breed::Cbr)).unwrap();
        p.push(Powl8Node::Choice {
            branches: [1, 2, 0, 0],
            len: 2,
        })
        .unwrap();
        p.push(Powl8Node::Loop {
            body: 1,
            max_iters: 3,
        })
        .unwrap();
        let s = serde_json::to_string(&powl8_to_jsonld(&p)).unwrap();
        assert!(s.contains("ccog:Choice"));
        assert!(s.contains("ccog:Loop"));
        assert!(s.contains("ccog:maxIters"));
    }
}
