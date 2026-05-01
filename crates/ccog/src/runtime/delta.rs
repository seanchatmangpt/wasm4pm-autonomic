//! ΔO detector — captures graph snapshots and computes set-difference deltas.
//!
//! Internally backed by [`PackedKeyTable`] keyed on a structured FNV-1a hash
//! of each triple, replacing the previous `HashSet<Triple>`. The public API
//! ([`GraphSnapshot`] / [`GraphDelta`]) is unchanged.

use anyhow::Result;
use oxigraph::model::{NamedOrBlankNode, Term, Triple};

use crate::graph::GraphStore;
use crate::utils::dense::{fnv1a_64, PackedKeyTable};

/// Structured FNV-1a-64 hash of a triple.
///
/// Layout (concatenated, fed to a single `fnv1a_64` invocation):
///
/// `subject_iri_or_blank_id | 0x00 | predicate_iri | 0x00 | object_repr`
///
/// where `object_repr` is:
/// - the IRI bytes for `Term::NamedNode`
/// - the blank-node id bytes for `Term::BlankNode`
/// - `b"L|" + lexical_form + 0x00 + datatype_or_lang_tag` for `Term::Literal`
///   (we encode the lexical form, a NUL separator, then the datatype IRI or
///   language tag — sufficient to disambiguate distinct literals; collisions
///   are handled by [`PackedKeyTable`]'s overwrite semantics, but ccog does
///   not rely on hash equality for triple identity beyond snapshot diffing).
///
/// The NUL byte separators prevent collision between e.g. "ab" + "c" and
/// "a" + "bc". This avoids `format!()` allocation per triple.
fn hash_triple(t: &Triple) -> u64 {
    // Worst-case path: pre-size a small Vec for the few-hundred-byte triple.
    let mut buf: Vec<u8> = Vec::with_capacity(128);

    // subject
    match &t.subject {
        NamedOrBlankNode::NamedNode(n) => buf.extend_from_slice(n.as_str().as_bytes()),
        NamedOrBlankNode::BlankNode(b) => buf.extend_from_slice(b.as_str().as_bytes()),
    }
    buf.push(0u8);

    // predicate
    buf.extend_from_slice(t.predicate.as_str().as_bytes());
    buf.push(0u8);

    // object
    match &t.object {
        Term::NamedNode(n) => buf.extend_from_slice(n.as_str().as_bytes()),
        Term::BlankNode(b) => buf.extend_from_slice(b.as_str().as_bytes()),
        Term::Literal(l) => {
            buf.extend_from_slice(b"L|");
            buf.extend_from_slice(l.value().as_bytes());
            buf.push(0u8);
            if let Some(lang) = l.language() {
                buf.extend_from_slice(b"@");
                buf.extend_from_slice(lang.as_bytes());
            } else {
                buf.extend_from_slice(l.datatype().as_str().as_bytes());
            }
        }
    }

    fnv1a_64(&buf)
}

/// Immutable snapshot of every triple in a `GraphStore`.
///
/// Backed internally by a [`PackedKeyTable`] keyed on [`hash_triple`].
/// `count` mirrors `table.len()` and is exposed via [`GraphSnapshot::len`].
#[derive(Clone, Debug)]
pub struct GraphSnapshot {
    /// Hash-indexed dense table of triples; key field is `()` because the
    /// hash itself is the lookup key.
    table: PackedKeyTable<(), Triple>,
    /// Cached entry count (equal to `table.len()`).
    count: usize,
}

impl GraphSnapshot {
    /// Capture all triples currently in `graph`.
    pub fn capture(graph: &GraphStore) -> Result<Self> {
        let triples = graph.all_triples()?;
        let mut table = PackedKeyTable::with_capacity(triples.len());
        for t in triples {
            let h = hash_triple(&t);
            table.insert(h, (), t);
        }
        let count = table.len();
        Ok(Self { table, count })
    }

    /// Number of triples in this snapshot.
    pub fn len(&self) -> usize {
        self.count
    }

    /// True when the snapshot contains no triples.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Set-difference between two snapshots: triples added and removed.
#[derive(Clone, Debug)]
pub struct GraphDelta {
    /// Triples present in `after` but not `before`.
    pub added: Vec<Triple>,
    /// Triples present in `before` but not `after`.
    pub removed: Vec<Triple>,
}

impl GraphDelta {
    /// Compute `after − before` (added) and `before − after` (removed).
    pub fn between(before: &GraphSnapshot, after: &GraphSnapshot) -> Self {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        for (hash, _, triple) in after.table.iter() {
            if before.table.get(*hash).is_none() {
                added.push(triple.clone());
            }
        }
        for (hash, _, triple) in before.table.iter() {
            if after.table.get(*hash).is_none() {
                removed.push(triple.clone());
            }
        }
        Self { added, removed }
    }

    /// Treat every triple as added (used on the first tick when no prior snapshot exists).
    pub fn all_added(snapshot: &GraphSnapshot) -> Self {
        let added: Vec<Triple> = snapshot.table.iter().map(|(_, _, t)| t.clone()).collect();
        Self {
            added,
            removed: Vec::new(),
        }
    }

    /// True when no triples were added or removed.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }

    /// Total count of changed triples.
    pub fn len(&self) -> usize {
        self.added.len() + self.removed.len()
    }
}
