//! CBR breed (Phase 9): Case-Based Reasoning reuse.
//!
//! CBR is admitted iff ≥1 `urn:ccog:Case` typed subject in the field graph
//! carries a `urn:ccog:vocab:caseFingerprint` triple whose object is a
//! BLAKE3 IRI of the form `urn:blake3:<hex>`. Admission is a pure
//! triple-pattern probe.
//!
//! Materialization is **not** on the `decide()` path. It walks the case
//! list, picks the lexicographically smallest case IRI, and emits a
//! [`ReusedCase`] with `similarity_q8 = 0xFF` (best-effort exact match)
//! and `adapted_construct` set to the BLAKE3 fingerprint IRI.

use anyhow::Result;
use oxigraph::model::{NamedNode, Term};

use crate::field::FieldContext;
use crate::graph::GraphIri;
use crate::verdict::ReusedCase;

const CASE_IRI: &str = "urn:ccog:Case";
const CASE_FINGERPRINT_PRED: &str = "urn:ccog:vocab:caseFingerprint";

/// Probe whether CBR is admissible against `field`.
///
/// Returns `true` iff there is at least one `?c rdf:type urn:ccog:Case`
/// triple AND `?c urn:ccog:vocab:caseFingerprint <urn:blake3:...>`.
pub fn admit(field: &FieldContext) -> Result<bool> {
    let case = NamedNode::new(CASE_IRI)?;
    let fingerprint = NamedNode::new(CASE_FINGERPRINT_PRED)?;
    let cases = field.graph.instances_of(&case)?;
    for c in &cases {
        for obj_iri in field.graph.objects_of(c, &fingerprint)? {
            if obj_iri.as_str().starts_with("urn:blake3:") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Materialize a [`ReusedCase`] from the lexicographically smallest
/// `urn:ccog:Case` carrying a `urn:blake3:` fingerprint.
///
/// **Not on the `decide()` hot path.** Returns `None` if no case has a
/// BLAKE3 fingerprint. `similarity_q8` is set to `0xFF` (interpreted as
/// "exact-match candidate"); downstream callers refine it via case
/// adaptation.
pub fn materialize(field: &FieldContext) -> Result<Option<ReusedCase>> {
    let case = NamedNode::new(CASE_IRI)?;
    let fingerprint = NamedNode::new(CASE_FINGERPRINT_PRED)?;
    let mut cases = field.graph.instances_of(&case)?;
    cases.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    for c in &cases {
        // Use generic quads_for_pattern via pairs_with_predicate filter? Use
        // objects_of which only returns NamedNodes — the IRI form we want.
        for obj_iri in field.graph.objects_of(c, &fingerprint)? {
            if obj_iri.as_str().starts_with("urn:blake3:") {
                let case_iri = GraphIri(c.clone());
                let adapted = GraphIri(obj_iri);
                return Ok(Some(ReusedCase {
                    case_iri,
                    similarity_q8: 0xFF,
                    adapted_construct: adapted,
                }));
            }
        }
    }
    // Defensive: also accept Term::Literal-objects if any (no-op since
    // objects_of only yields NamedNodes; kept for documentation).
    let _ = Term::NamedNode;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field_with(triples: &str) -> FieldContext {
        let mut f = FieldContext::new("cbr");
        f.load_field_state(triples).expect("load");
        f
    }

    #[test]
    fn cbr_admitted_when_case_library_present() -> Result<()> {
        let f = field_with(
            "<http://example.org/case1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Case> .\n\
             <http://example.org/case1> <urn:ccog:vocab:caseFingerprint> <urn:blake3:deadbeef> .\n",
        );
        assert!(admit(&f)?);
        let r = materialize(&f)?.expect("must yield a case");
        assert_eq!(r.case_iri.as_str(), "http://example.org/case1");
        assert_eq!(r.adapted_construct.as_str(), "urn:blake3:deadbeef");
        assert_eq!(r.similarity_q8, 0xFF);
        Ok(())
    }

    #[test]
    fn cbr_denied_on_empty_library() -> Result<()> {
        let f = FieldContext::new("cbr-empty");
        assert!(!admit(&f)?);
        assert!(materialize(&f)?.is_none());
        Ok(())
    }

    #[test]
    fn cbr_denied_when_case_lacks_blake3_fingerprint() -> Result<()> {
        // Case has rdf:type but fingerprint object is not a urn:blake3 IRI.
        let f = field_with(
            "<http://example.org/case1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:ccog:Case> .\n\
             <http://example.org/case1> <urn:ccog:vocab:caseFingerprint> <http://example.org/something> .\n",
        );
        assert!(!admit(&f)?);
        Ok(())
    }
}
