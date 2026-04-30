//! Compiled field snapshot for nanosecond hook bark (Phase 4 Stage 1).
//!
//! Builds once per `fire_matching` call by single-pass
//! `quads_for_pattern(None,None,None,None)` enumeration. Built-in hooks read
//! the snapshot via O(1) `HashMap` lookups instead of repeated graph walks,
//! amortizing the parser/iterator cost across all hooks in a fire.
//!
//! # Indices
//!
//! - `instances_by_class` — `rdf:type` → subjects
//! - `subject_predicate_present` — `(s, p)` existence
//! - `label_index` — lowercase `skos:prefLabel`/`skos:altLabel`/`schema:name` → subjects
//! - `by_predicate` — predicate → `(subject, object)` pairs

use crate::field::FieldContext;
use anyhow::Result;
use oxigraph::model::{NamedNode, Term};
use std::collections::{HashMap, HashSet};

/// One-pass indexed snapshot of the field's graph state.
///
/// Built from a single full-graph traversal. Provides O(1) lookups for
/// the patterns built-in hooks need: class instances, subject-predicate
/// presence, lowercase label index, and predicate → pairs.
#[derive(Debug, Default)]
pub struct CompiledFieldSnapshot {
    instances_by_class: HashMap<String, Vec<NamedNode>>,
    subject_predicate_present: HashSet<(String, String)>,
    label_index: HashMap<String, Vec<NamedNode>>,
    by_predicate: HashMap<String, Vec<(NamedNode, Term)>>,
}

impl CompiledFieldSnapshot {
    /// Build a snapshot from the field's graph in a single pass.
    ///
    /// Iterates `field.graph.all_triples()` exactly once and populates all
    /// indices. Blank-node subjects are skipped (subject_predicate_present
    /// requires NamedNode subjects).
    pub fn from_field(field: &FieldContext) -> Result<Self> {
        const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
        const SKOS_PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
        const SKOS_ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";
        const SCHEMA_NAME: &str = "https://schema.org/name";

        let mut snap = CompiledFieldSnapshot::default();

        for triple in field.graph.all_triples()? {
            let subj = match triple.subject {
                oxigraph::model::NamedOrBlankNode::NamedNode(n) => n,
                oxigraph::model::NamedOrBlankNode::BlankNode(_) => continue,
            };
            let pred = triple.predicate;
            let obj = triple.object;

            let pred_iri = pred.as_str().to_string();
            let subj_iri = subj.as_str().to_string();

            snap.subject_predicate_present
                .insert((subj_iri, pred_iri.clone()));

            if pred.as_str() == RDF_TYPE {
                if let Term::NamedNode(class) = &obj {
                    snap.instances_by_class
                        .entry(class.as_str().to_string())
                        .or_default()
                        .push(subj.clone());
                }
            }

            if matches!(pred.as_str(), SKOS_PREF_LABEL | SKOS_ALT_LABEL | SCHEMA_NAME) {
                if let Term::Literal(lit) = &obj {
                    let lc = lit.value().to_lowercase();
                    let bucket = snap.label_index.entry(lc).or_default();
                    if !bucket.iter().any(|n| n == &subj) {
                        bucket.push(subj.clone());
                    }
                }
            }

            snap.by_predicate
                .entry(pred_iri)
                .or_default()
                .push((subj, obj));
        }

        Ok(snap)
    }

    /// Subjects with `rdf:type class`. O(1) lookup.
    pub fn instances_of(&self, class: &NamedNode) -> &[NamedNode] {
        self.instances_by_class
            .get(class.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// True iff `(subject, predicate, ?)` exists in the graph. O(1) lookup.
    pub fn has_value_for(&self, subject: &NamedNode, predicate: &NamedNode) -> bool {
        self.subject_predicate_present.contains(&(
            subject.as_str().to_string(),
            predicate.as_str().to_string(),
        ))
    }

    /// Subjects whose `skos:prefLabel`/`skos:altLabel`/`schema:name` lowercases to `lowercase`.
    pub fn lookup_label(&self, lowercase: &str) -> &[NamedNode] {
        self.label_index
            .get(lowercase)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// All `(subject, object)` pairs for the given predicate. O(1) lookup.
    pub fn pairs_with_predicate(&self, predicate: &NamedNode) -> &[(NamedNode, Term)] {
        self.by_predicate
            .get(predicate.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// True iff any triple uses the given predicate. O(1) lookup.
    pub fn has_any_with_predicate(&self, predicate: &NamedNode) -> bool {
        self.by_predicate
            .get(predicate.as_str())
            .is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_indexes_instances_by_class() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n\
             <http://example.org/d2> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://schema.org/DigitalDocument> .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let class = NamedNode::new("https://schema.org/DigitalDocument")?;
        assert_eq!(snap.instances_of(&class).len(), 2);
        Ok(())
    }

    #[test]
    fn snapshot_tracks_subject_predicate_presence() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/ns/prov#value> \"x\" .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let s = NamedNode::new("http://example.org/d1")?;
        let p = NamedNode::new("http://www.w3.org/ns/prov#value")?;
        assert!(snap.has_value_for(&s, &p));
        let p2 = NamedNode::new("http://www.w3.org/ns/prov#wasGeneratedBy")?;
        assert!(!snap.has_value_for(&s, &p2));
        Ok(())
    }

    #[test]
    fn snapshot_indexes_labels_lowercase() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"Hello World\" .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        assert_eq!(snap.lookup_label("hello world").len(), 1);
        assert_eq!(snap.lookup_label("HELLO").len(), 0);
        Ok(())
    }

    #[test]
    fn snapshot_has_any_with_predicate() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"x\" .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let p = NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")?;
        assert!(snap.has_any_with_predicate(&p));
        let p2 = NamedNode::new("http://www.w3.org/2004/02/skos/core#altLabel")?;
        assert!(!snap.has_any_with_predicate(&p2));
        Ok(())
    }

    #[test]
    fn snapshot_pairs_with_predicate() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/c1> <http://www.w3.org/2004/02/skos/core#prefLabel> \"a\" .\n\
             <http://example.org/c2> <http://www.w3.org/2004/02/skos/core#prefLabel> \"b\" .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let p = NamedNode::new("http://www.w3.org/2004/02/skos/core#prefLabel")?;
        assert_eq!(snap.pairs_with_predicate(&p).len(), 2);
        Ok(())
    }
}
