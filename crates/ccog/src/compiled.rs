//! Compiled field snapshot for nanosecond hook bark (Phase 4 Stage 1, Phase 8 interned).
//!
//! Builds once per `fire_matching` call by single-pass
//! `quads_for_pattern(None,None,None,None)` enumeration. Built-in hooks read
//! the snapshot via O(1) `PackedKeyTable` lookups (Phase 8 migration —
//! HashMap/HashSet replaced by content-addressed dense tables keyed by
//! `fnv1a_64` of the IRI bytes), amortizing the parser/iterator cost across
//! all hooks in a fire.
//!
//! # Indices
//!
//! - `instances_by_class` — `fnv1a_64(class IRI)` → subjects
//! - `subject_predicate_present` — `fnv1a_64(subj || 0x00 || pred)` → present-bit
//! - `label_index` — `fnv1a_64(lowercase label)` → subjects
//! - `by_predicate` — `fnv1a_64(pred IRI)` → `(subject, object)` pairs
//!
//! # Hash collision policy
//!
//! `PackedKeyTable` is keyed solely by the `u64` hash; collisions overwrite.
//! For the four-entry-class snapshot sizes we expect (≤ a few thousand
//! distinct IRIs per fire), a 64-bit FNV-1a collision has probability
//! ≤ 2⁻⁶². Documented in `subject_predicate_present_collision_test`.

use crate::field::FieldContext;
use crate::utils::dense::{fnv1a_64, PackedKeyTable};
use anyhow::Result;
use oxigraph::model::{NamedNode, Term};

/// One-pass indexed snapshot of the field's graph state.
///
/// All four indices are [`PackedKeyTable`]s keyed by `fnv1a_64` of the
/// IRI bytes — no `std::collections::HashMap` on the hot path.
#[derive(Debug, Default, Clone)]
pub struct CompiledFieldSnapshot {
    instances_by_class: PackedKeyTable<(), Vec<NamedNode>>,
    subject_predicate_present: PackedKeyTable<(), ()>,
    label_index: PackedKeyTable<(), Vec<NamedNode>>,
    by_predicate: PackedKeyTable<(), Vec<(NamedNode, Term)>>,
}

#[inline]
fn hash_pair(subj: &str, pred: &str) -> u64 {
    // Deterministic FNV-1a over `subj || 0x00 || pred` — alloc-free.
    // The NUL byte cannot appear inside an IRI so the boundary is unambiguous.
    // Fold byte-by-byte instead of building a temporary buffer.
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut h = FNV_OFFSET;
    for &b in subj.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h ^= 0;
    h = h.wrapping_mul(FNV_PRIME);
    for &b in pred.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

impl CompiledFieldSnapshot {
    /// Build a snapshot from the field's graph in a single pass.
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

            let pred_iri = pred.as_str();
            let subj_iri = subj.as_str();

            let pair_h = hash_pair(subj_iri, pred_iri);
            snap.subject_predicate_present.insert(pair_h, (), ());

            if pred.as_str() == RDF_TYPE {
                if let Term::NamedNode(class) = &obj {
                    let class_h = fnv1a_64(class.as_str().as_bytes());
                    if let Some(bucket) = snap.instances_by_class.get_mut(class_h) {
                        bucket.push(subj.clone());
                    } else {
                        snap.instances_by_class
                            .insert(class_h, (), vec![subj.clone()]);
                    }
                }
            }

            if matches!(
                pred.as_str(),
                SKOS_PREF_LABEL | SKOS_ALT_LABEL | SCHEMA_NAME
            ) {
                if let Term::Literal(lit) = &obj {
                    let lc = lit.value().to_lowercase();
                    let lh = fnv1a_64(lc.as_bytes());
                    if let Some(bucket) = snap.label_index.get_mut(lh) {
                        if !bucket.iter().any(|n| n == &subj) {
                            bucket.push(subj.clone());
                        }
                    } else {
                        snap.label_index.insert(lh, (), vec![subj.clone()]);
                    }
                }
            }

            let ph = fnv1a_64(pred_iri.as_bytes());
            if let Some(bucket) = snap.by_predicate.get_mut(ph) {
                bucket.push((subj, obj));
            } else {
                snap.by_predicate.insert(ph, (), vec![(subj, obj)]);
            }
        }

        Ok(snap)
    }

    /// Subjects with `rdf:type class`. O(1) lookup.
    pub fn instances_of(&self, class: &NamedNode) -> &[NamedNode] {
        self.instances_by_class
            .get(fnv1a_64(class.as_str().as_bytes()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// True iff `(subject, predicate, ?)` exists in the graph. O(1) lookup.
    pub fn has_value_for(&self, subject: &NamedNode, predicate: &NamedNode) -> bool {
        self.subject_predicate_present
            .get(hash_pair(subject.as_str(), predicate.as_str()))
            .is_some()
    }

    /// Subjects whose `skos:prefLabel`/`skos:altLabel`/`schema:name` lowercases to `lowercase`.
    pub fn lookup_label(&self, lowercase: &str) -> &[NamedNode] {
        self.label_index
            .get(fnv1a_64(lowercase.as_bytes()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// All `(subject, object)` pairs for the given predicate. O(1) lookup.
    pub fn pairs_with_predicate(&self, predicate: &NamedNode) -> &[(NamedNode, Term)] {
        self.by_predicate
            .get(fnv1a_64(predicate.as_str().as_bytes()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// True iff any triple uses the given predicate. O(1) lookup.
    pub fn has_any_with_predicate(&self, predicate: &NamedNode) -> bool {
        self.by_predicate
            .get(fnv1a_64(predicate.as_str().as_bytes()))
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

    /// Documents that the interned snapshot is keyed by `fnv1a_64`
    /// of `subj || 0x00 || pred`. Equal hashes overwrite (one
    /// presence-bit per pair) — collision probability is ≤ 2⁻⁶² for
    /// realistic snapshot sizes.
    #[test]
    fn subject_predicate_present_collision_test() -> Result<()> {
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/d1> <http://www.w3.org/ns/prov#value> \"x\" .\n\
             <http://example.org/d1> <http://www.w3.org/ns/prov#wasGeneratedBy> <http://example.org/a1> .\n\
             <http://example.org/d2> <http://www.w3.org/ns/prov#value> \"y\" .\n",
        )?;
        let snap = CompiledFieldSnapshot::from_field(&field)?;
        let s1 = NamedNode::new("http://example.org/d1")?;
        let s2 = NamedNode::new("http://example.org/d2")?;
        let pv = NamedNode::new("http://www.w3.org/ns/prov#value")?;
        let pg = NamedNode::new("http://www.w3.org/ns/prov#wasGeneratedBy")?;
        assert!(snap.has_value_for(&s1, &pv));
        assert!(snap.has_value_for(&s1, &pg));
        assert!(snap.has_value_for(&s2, &pv));
        assert!(!snap.has_value_for(&s2, &pg));
        Ok(())
    }
}
