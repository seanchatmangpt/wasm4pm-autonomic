//! SOAR breed (Phase 9 — gated stub): chunking via shape-fingerprint clusters.
//!
//! SOAR is admitted **only** when the field graph carries a
//! `urn:ccog:trace-history` resource with ≥3 `prov:wasGeneratedBy` activities
//! sharing a single `urn:ccog:shapeFingerprint` literal. Until Phase 7's
//! trace-history persistence lands, this admission probe will routinely
//! return `false` against real fields — that is intentional.
//!
//! Admission is a **pure graph probe**: count distinct activities reachable
//! via `prov:wasGeneratedBy` from any subject typed as
//! `urn:ccog:trace-history`, then count how many share a single
//! `urn:ccog:shapeFingerprint` literal. No in-memory state, no SPARQL
//! parsing, no per-call caches.

use anyhow::Result;
use oxigraph::model::{NamedNode, Term};
use smallvec::SmallVec;

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::MacroOperator;

const TRACE_HISTORY_IRI: &str = "urn:ccog:trace-history";
const PROV_WAS_GENERATED_BY: &str = "http://www.w3.org/ns/prov#wasGeneratedBy";
const SHAPE_FINGERPRINT_PRED: &str = "urn:ccog:vocab:shapeFingerprint";

/// Probe whether SOAR is admissible against `field`.
///
/// Returns `true` iff:
///   1. The graph has at least one `?h rdf:type urn:ccog:trace-history` triple, AND
///   2. There exist ≥3 distinct `prov:wasGeneratedBy` objects (activities)
///      reachable from such a `?h` (the activities themselves are subjects), AND
///   3. ≥3 of those activities share a single `urn:ccog:vocab:shapeFingerprint`
///      literal value.
///
/// All probes are direct `pattern_exists` / `instances_of` walks; no
/// hashmap is materialized on the hot path.
pub fn admit(field: &FieldContext) -> Result<bool> {
    let trace_history = NamedNode::new(TRACE_HISTORY_IRI)?;
    let was_generated_by = NamedNode::new(PROV_WAS_GENERATED_BY)?;
    let fingerprint = NamedNode::new(SHAPE_FINGERPRINT_PRED)?;

    let history_subjects = field.graph.instances_of(&trace_history)?;
    if history_subjects.is_empty() {
        return Ok(false);
    }

    // Collect all activities `?a` such that some `?h` in history_subjects has
    // `?h prov:wasGeneratedBy ?a`. Bounded scan; no hashmap.
    let mut activities: SmallVec<[NamedNode; 16]> = SmallVec::new();
    for h in &history_subjects {
        for a in field.graph.objects_of(h, &was_generated_by)? {
            if !activities.iter().any(|x| x.as_str() == a.as_str()) {
                activities.push(a);
            }
        }
    }
    if activities.len() < 3 {
        return Ok(false);
    }

    // For each fingerprint literal, count activities carrying it. SOAR is
    // admitted as soon as any fingerprint reaches 3 activities. Bounded
    // double loop over `pairs_with_predicate` — acceptable because trace
    // history is bounded by construction.
    let mut counted: SmallVec<[(String, u8); 8]> = SmallVec::new();
    for (subject, term) in field.graph.pairs_with_predicate(&fingerprint)? {
        let lit = match term {
            Term::Literal(l) => l.value().to_string(),
            _ => continue,
        };
        if !activities.iter().any(|a| a.as_str() == subject.as_str()) {
            continue;
        }
        match counted.iter_mut().find(|(k, _)| *k == lit) {
            Some((_, c)) => {
                *c = c.saturating_add(1);
                if *c >= 3 {
                    return Ok(true);
                }
            }
            None => counted.push((lit, 1)),
        }
    }
    Ok(false)
}

/// Materialize a [`MacroOperator`] from the most-frequent fingerprint.
///
/// **Not on the `decide()` hot path.** Returns `None` if no fingerprint
/// reaches 3 occurrences. The returned `compressed_breeds` field is
/// always 0 in this Phase-9 stub — Phase 7 trace-history persistence is
/// required to encode the actual breed sequence. Until then, the slot
/// is reserved.
pub fn materialize(field: &FieldContext) -> Result<Option<MacroOperator>> {
    if !admit(field)? {
        return Ok(None);
    }
    let fingerprint = NamedNode::new(SHAPE_FINGERPRINT_PRED)?;
    let mut best: Option<(String, u8)> = None;
    let mut counts: SmallVec<[(String, u8); 8]> = SmallVec::new();
    for (_subject, term) in field.graph.pairs_with_predicate(&fingerprint)? {
        let lit = match term {
            Term::Literal(l) => l.value().to_string(),
            _ => continue,
        };
        match counts.iter_mut().find(|(k, _)| *k == lit) {
            Some((_, c)) => *c = c.saturating_add(1),
            None => counts.push((lit, 1)),
        }
    }
    for (k, c) in &counts {
        if best.as_ref().map_or(true, |(_, bc)| *c > *bc) {
            best = Some((k.clone(), *c));
        }
    }
    let (label, replay_count) = match best {
        Some(b) => b,
        None => return Ok(None),
    };
    let hash = blake3::hash(label.as_bytes());
    let urn = format!("urn:blake3:{}", hash.to_hex());
    Ok(Some(MacroOperator {
        fingerprint: GraphIri::from_iri(&urn)?,
        compressed_breeds: 0,
        replay_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_with(triples: &str) -> FieldContext {
        let mut f = FieldContext::new("soar");
        f.load_field_state(triples).expect("load");
        f
    }

    #[test]
    fn soar_admitted_with_three_matching_traces() -> Result<()> {
        let f = field_with(
            "<http://example.org/h1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:trace-history> .\n\
             <http://example.org/h1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a1> .\n\
             <http://example.org/h1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a2> .\n\
             <http://example.org/h1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a3> .\n\
             <http://example.org/a1> <urn:ccog:vocab:shapeFingerprint> \"shape-X\" .\n\
             <http://example.org/a2> <urn:ccog:vocab:shapeFingerprint> \"shape-X\" .\n\
             <http://example.org/a3> <urn:ccog:vocab:shapeFingerprint> \"shape-X\" .\n",
        );
        assert!(admit(&f)?);
        Ok(())
    }

    #[test]
    fn soar_denied_when_history_diverges() -> Result<()> {
        // Three activities, but only 2 share a fingerprint.
        let f = field_with(
            "<http://example.org/h1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:trace-history> .\n\
             <http://example.org/h1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a1> .\n\
             <http://example.org/h1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a2> .\n\
             <http://example.org/h1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a3> .\n\
             <http://example.org/a1> <urn:ccog:vocab:shapeFingerprint> \"shape-X\" .\n\
             <http://example.org/a2> <urn:ccog:vocab:shapeFingerprint> \"shape-X\" .\n\
             <http://example.org/a3> <urn:ccog:vocab:shapeFingerprint> \"shape-Y\" .\n",
        );
        assert!(!admit(&f)?);
        Ok(())
    }

    #[test]
    fn soar_denied_on_empty_field() -> Result<()> {
        let f = FieldContext::new("soar-empty");
        assert!(!admit(&f)?);
        Ok(())
    }
}
