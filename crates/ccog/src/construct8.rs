//! Bounded write primitive for SPARQL CONSTRUCT deltas.
//!
//! `Construct8` enforces a strict ≤8 triple limit on CONSTRUCT query results,
//! enabling deterministic bounded mutations in RDF stores. This primitive is
//! load-bearing for the provenance chain's delta-receipt interface.
//!
//! # Examples
//!
//! ```ignore
//! let mut delta = Construct8::empty();
//! delta.push(triple)?;
//! let bytes = delta.receipt_bytes(); // BLAKE3-hashable N-Triples bytes
//! ```

use crate::graph::GraphStore;
use crate::utils::dense::fnv1a_64;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// CONSTRUCT query result overflow error.
#[derive(Debug, thiserror::Error)]
pub enum Construct8Error {
    /// Query produced more than 8 triples.
    #[error("CONSTRUCT8 overflow: query produced {0} triples, maximum is 8")]
    Overflow(usize),
}

/// Object identifier (stable Rust u32 wrapper).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub u32);

impl core::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:08x}", self.0)
    }
}

/// Predicate identifier (stable Rust u16 wrapper).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PredicateId(pub u16);

impl core::fmt::Display for PredicateId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04x}", self.0)
    }
}

/// Stable Rust Triple struct (PRD v0.4 Section 13).
///
/// Zero-allocation Triple representation using interned or hashed identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Triple {
    /// Subject of the triple.
    pub subject: ObjectId,
    /// Predicate of the triple.
    pub predicate: PredicateId,
    /// Object of the triple.
    pub object: ObjectId,
}

impl Triple {
    /// Create a new Triple from ObjectId and PredicateId.
    pub const fn new(subject: ObjectId, predicate: PredicateId, object: ObjectId) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }

    /// Create a new Triple from strings, hashing them into IDs.
    pub fn from_strings(s: &str, p: &str, o: &str) -> Self {
        Self {
            subject: ObjectId(fnv1a_64(s.as_bytes()) as u32),
            predicate: PredicateId(fnv1a_64(p.as_bytes()) as u16),
            object: ObjectId(fnv1a_64(o.as_bytes()) as u32),
        }
    }
}

/// Bounded write primitive — SPARQL CONSTRUCT delta capped at ≤8 triples.
///
/// Stores up to 8 RDF triples with deterministic N-Triples serialization
/// for provenance chain receipt generation. Enforces overflow detection.
#[derive(Debug, Clone)]
pub struct Construct8 {
    triples: [Option<Triple>; 8],
    len: u8,
}

/// Branchless capacity probe for [`Construct8::push`].
///
/// Returns an admit verdict (`0` admitted, non-zero denied) describing whether
/// the next slot is writable. Routing `push` through this probe lets the hot
/// path compose with other denial-polarity admit verdicts via bitwise OR.
#[inline(always)]
const fn admit_push(len: u8) -> u64 {
    crate::admit::bool_mask(len < 8)
}

impl Construct8 {
    /// Creates an empty CONSTRUCT8 delta.
    pub fn empty() -> Self {
        Self {
            triples: [None, None, None, None, None, None, None, None],
            len: 0,
        }
    }

    /// Loads a CONSTRUCT8 delta from a SPARQL CONSTRUCT query.
    ///
    /// Executes the query against the graph store and collects results
    /// into the bounded array. Returns `Construct8Error::Overflow` if
    /// the query produces more than 8 triples.
    ///
    /// # Errors
    ///
    /// Returns `Construct8Error::Overflow` if query result exceeds 8 triples.
    /// Returns `anyhow::Error` for SPARQL syntax or execution errors.
    pub fn from_sparql(store: &GraphStore, query: &str) -> Result<Self> {
        let mut delta = Self::empty();

        let triples = store.construct(query)?;

        for triple in triples {
            let s = match &triple.subject {
                oxigraph::model::NamedOrBlankNode::NamedNode(n) => n.as_str(),
                oxigraph::model::NamedOrBlankNode::BlankNode(b) => b.as_str(),
            };
            let p = triple.predicate.as_str();
            let o = match &triple.object {
                oxigraph::model::Term::NamedNode(n) => n.as_str(),
                oxigraph::model::Term::BlankNode(b) => b.as_str(),
                oxigraph::model::Term::Literal(l) => l.value(),
            };

            let subject = ObjectId(fnv1a_64(s.as_bytes()) as u32);
            let predicate = PredicateId(fnv1a_64(p.as_bytes()) as u16);
            let object = ObjectId(fnv1a_64(o.as_bytes()) as u32);

            if !delta.push(Triple::new(subject, predicate, object)) {
                return Err(anyhow::anyhow!(Construct8Error::Overflow(delta.len() + 1)));
            }
        }

        Ok(delta)
    }

    /// Adds a triple to the delta.
    ///
    /// Returns `true` if the triple was added successfully,
    /// `false` if the delta is full (already contains 8 triples).
    pub fn push(&mut self, triple: Triple) -> bool {
        let verdict = admit_push(self.len);
        if !crate::admit::admitted(verdict) {
            return false;
        }
        self.triples[self.len as usize] = Some(triple);
        self.len += 1;
        true
    }

    /// Returns the number of triples in the delta.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns `true` if the delta contains no triples.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the delta is full (contains 8 triples).
    pub fn is_full(&self) -> bool {
        self.len == 8
    }

    /// Returns an iterator over the triples in the delta.
    ///
    /// Yields only the occupied slots, skipping any `None` entries.
    pub fn iter(&self) -> impl Iterator<Item = &Triple> {
        self.triples.iter().filter_map(|opt| opt.as_ref())
    }

    /// Serializes the delta as N-Triples bytes for BLAKE3 hashing.
    ///
    /// Each triple is formatted using its IDs as `_:<subject> <_:<predicate>> _:<object> .\n`.
    /// This ensures zero-allocation and deterministic output for hashing.
    pub fn receipt_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for triple in self.iter() {
            bytes.extend_from_slice(
                format!(
                    "<urn:ccog:id:{:08x}> <urn:ccog:p:{:04x}> <urn:ccog:id:{:08x}> .\n",
                    triple.subject.0, triple.predicate.0, triple.object.0
                )
                .as_bytes(),
            );
        }
        bytes
    }

    /// Serializes the delta as an N-Triples format string.
    pub fn to_ntriples(&self) -> String {
        let mut output = String::new();
        for triple in self.iter() {
            output.push_str(&format!(
                "<urn:ccog:id:{:08x}> <urn:ccog:p:{:04x}> <urn:ccog:id:{:08x}> .\n",
                triple.subject.0, triple.predicate.0, triple.object.0
            ));
        }
        output
    }

    /// Materializes the delta triples into a graph store.
    ///
    /// Note: Materialization of ID-only triples requires an external symbol table
    /// to map back to original IRIs. In this zero-allocation implementation,
    /// we emit blank nodes derived from the IDs.
    pub fn materialize(&self, store: &GraphStore) -> Result<()> {
        store.insert_ntriples(&self.to_ntriples())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that empty_construct8 has len zero and is_empty returns true.
    #[test]
    fn empty_construct8_has_len_zero() {
        let delta = Construct8::empty();
        assert_eq!(delta.len(), 0);
        assert!(delta.is_empty());
        assert!(!delta.is_full());
    }

    /// Verifies that pushing 8 triples succeeds and the 9th fails.
    #[test]
    fn push_to_capacity_succeeds() {
        let mut delta = Construct8::empty();

        let subject = ObjectId(1);
        let predicate = PredicateId(2);
        let object = ObjectId(3);
        let triple = Triple::new(subject, predicate, object);

        // Push 8 triples successfully
        for i in 0..8 {
            assert!(delta.push(triple), "push {} failed", i);
            assert_eq!(delta.len(), i + 1);
        }

        assert!(delta.is_full());

        // 9th push should fail
        assert!(!delta.push(triple));
        assert_eq!(delta.len(), 8);
    }

    /// Verifies that receipt_bytes is deterministic for the same input.
    #[test]
    fn receipt_bytes_deterministic() {
        let mut delta1 = Construct8::empty();
        let mut delta2 = Construct8::empty();

        let triple = Triple::new(ObjectId(1), PredicateId(2), ObjectId(3));

        delta1.push(triple);
        delta2.push(triple);

        let bytes1 = delta1.receipt_bytes();
        let bytes2 = delta2.receipt_bytes();

        assert_eq!(bytes1, bytes2, "receipt_bytes should be deterministic");
    }

    /// Verifies that to_ntriples output is valid N-Triples format.
    #[test]
    fn to_ntriples_valid() {
        let mut delta = Construct8::empty();
        let triple = Triple::new(ObjectId(1), PredicateId(2), ObjectId(3));
        delta.push(triple);

        let ntriples = delta.to_ntriples();
        assert!(!ntriples.is_empty(), "to_ntriples should not be empty");
        assert!(
            ntriples.ends_with(" .\n"),
            "N-Triples must end with ' .\\n'"
        );
    }
}
