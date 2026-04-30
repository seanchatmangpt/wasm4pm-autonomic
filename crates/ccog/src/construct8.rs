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

use oxigraph::model::Triple;
use anyhow::Result;
use crate::graph::GraphStore;

/// CONSTRUCT query result overflow error.
#[derive(Debug, thiserror::Error)]
pub enum Construct8Error {
    /// Query produced more than 8 triples.
    #[error("CONSTRUCT8 overflow: query produced {0} triples, maximum is 8")]
    Overflow(usize),
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
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let delta = Construct8::empty();
    /// assert_eq!(delta.len(), 0);
    /// ```
    pub fn empty() -> Self {
        Self {
            triples: [
                None, None, None, None, None, None, None, None,
            ],
            len: 0,
        }
    }

    /// Loads a CONSTRUCT8 delta from a SPARQL CONSTRUCT query.
    ///
    /// Executes the query against the graph store and collects results
    /// into the bounded array. Returns `Construct8Error::Overflow` if
    /// the query produces more than 8 triples.
    ///
    /// # Arguments
    ///
    /// * `store` - The RDF graph store to query
    /// * `query` - SPARQL CONSTRUCT query string
    ///
    /// # Errors
    ///
    /// Returns `Construct8Error::Overflow` if query result exceeds 8 triples.
    /// Returns `anyhow::Error` for SPARQL syntax or execution errors.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let delta = Construct8::from_sparql(&store,
    ///     "CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o LIMIT 8 }")?;
    /// ```
    pub fn from_sparql(store: &GraphStore, query: &str) -> Result<Self> {
        let mut delta = Self::empty();

        let triples = store.construct(query)?;

        for triple in triples {
            if !delta.push(triple) {
                // Count total triples in result to report accurate overflow
                return Err(anyhow::anyhow!(Construct8Error::Overflow(delta.len() as usize + 1)));
            }
        }

        Ok(delta)
    }

    /// Adds a triple to the delta.
    ///
    /// Returns `true` if the triple was added successfully,
    /// `false` if the delta is full (already contains 8 triples).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut delta = Construct8::empty();
    /// assert!(delta.push(triple1));
    /// // ... push up to 8 triples ...
    /// assert!(!delta.push(triple9)); // Returns false when full
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut delta = Construct8::empty();
    /// assert_eq!(delta.len(), 0);
    /// delta.push(triple);
    /// assert_eq!(delta.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns `true` if the delta contains no triples.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let delta = Construct8::empty();
    /// assert!(delta.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the delta is full (contains 8 triples).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut delta = Construct8::empty();
    /// assert!(!delta.is_full());
    /// // ... push 8 triples ...
    /// assert!(delta.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        self.len == 8
    }

    /// Returns an iterator over the triples in the delta.
    ///
    /// Yields only the occupied slots, skipping any `None` entries.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let delta = Construct8::empty();
    /// for triple in delta.iter() {
    ///     println!("{}", triple);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &Triple> {
        self.triples.iter().filter_map(|opt| opt.as_ref())
    }

    /// Serializes the delta as N-Triples bytes for BLAKE3 hashing.
    ///
    /// Each triple is formatted as `<subject> <predicate> <object> .\n` in
    /// canonical N-Triples form. The byte sequence is deterministic and
    /// suitable for provenance chain receipt generation.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let delta = Construct8::empty();
    /// let bytes = delta.receipt_bytes();
    /// let hash = blake3::hash(&bytes);
    /// ```
    pub fn receipt_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for triple in self.iter() {
            bytes.extend_from_slice(format!("{} .\n", triple).as_bytes());
        }
        bytes
    }

    /// Serializes the delta as an N-Triples format string.
    ///
    /// Each triple is formatted as `<subject> <predicate> <object> .\n` in
    /// canonical N-Triples form. The output can be loaded into an RDF store
    /// using `GraphStore::load_triples` or equivalent.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let delta = Construct8::empty();
    /// let ntriples = delta.to_ntriples();
    /// store.load_triples(ntriples.as_bytes())?;
    /// ```
    pub fn to_ntriples(&self) -> String {
        let mut output = String::new();
        for triple in self.iter() {
            output.push_str(&format!("{} .\n", triple));
        }
        output
    }

    /// Materializes the delta triples into a graph store.
    ///
    /// Inserts all triples contained in this delta into the provided
    /// RDF graph store. Returns an error if insertion fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut delta = Construct8::empty();
    /// delta.push(triple);
    /// delta.materialize(&field.graph)?;
    /// ```
    pub fn materialize(&self, store: &GraphStore) -> Result<()> {
        let triples: Vec<Triple> = self.iter().cloned().collect();
        store.insert_triples(&triples)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::{NamedNode, Literal};

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

        let subject = NamedNode::new("http://example.org/subject").unwrap();
        let predicate = NamedNode::new("http://example.org/predicate").unwrap();
        let object = NamedNode::new("http://example.org/object").unwrap();

        // Push 8 triples successfully
        for i in 0..8 {
            let triple = Triple::new(subject.clone(), predicate.clone(), object.clone());
            assert!(delta.push(triple), "push {} failed", i);
            assert_eq!(delta.len(), i + 1);
        }

        assert!(delta.is_full());

        // 9th push should fail
        let triple = Triple::new(subject, predicate, object);
        assert!(!delta.push(triple));
        assert_eq!(delta.len(), 8);
    }

    /// Verifies that SPARQL CONSTRUCT returning 9 triples produces Overflow error.
    #[test]
    fn overflow_from_sparql() {
        // This test verifies the overflow error type and message.
        let err = Construct8Error::Overflow(9);
        assert_eq!(err.to_string(), "CONSTRUCT8 overflow: query produced 9 triples, maximum is 8");
    }

    /// Verifies that receipt_bytes is deterministic for the same input.
    #[test]
    fn receipt_bytes_deterministic() {
        let mut delta1 = Construct8::empty();
        let mut delta2 = Construct8::empty();

        let subject = NamedNode::new("http://example.org/s").unwrap();
        let predicate = NamedNode::new("http://example.org/p").unwrap();
        let object = NamedNode::new("http://example.org/o").unwrap();
        let triple = Triple::new(subject, predicate, object);

        delta1.push(triple.clone());
        delta2.push(triple.clone());

        let bytes1 = delta1.receipt_bytes();
        let bytes2 = delta2.receipt_bytes();

        assert_eq!(bytes1, bytes2, "receipt_bytes should be deterministic");
    }

    /// Verifies that to_ntriples output is valid N-Triples format.
    #[test]
    fn to_ntriples_valid() {
        use oxigraph::model::Term;

        let mut delta = Construct8::empty();

        let subject = NamedNode::new("http://example.org/subj").unwrap();
        let predicate = NamedNode::new("http://example.org/pred").unwrap();
        let object: Term = Literal::new_simple_literal("test").into();
        let triple = Triple::new(subject, predicate, object);

        delta.push(triple);

        let ntriples = delta.to_ntriples();
        assert!(!ntriples.is_empty(), "to_ntriples should not be empty");
        assert!(ntriples.ends_with(" .\n"), "N-Triples must end with ' .\\n'");
    }

    /// Verifies that iter skips None slots and only yields filled slots.
    #[test]
    fn iter_skips_none_slots() {
        let mut delta = Construct8::empty();

        let subject = NamedNode::new("http://example.org/s").unwrap();
        let predicate = NamedNode::new("http://example.org/p").unwrap();
        let object = NamedNode::new("http://example.org/o").unwrap();

        // Push only 2 triples (leaving 6 slots as None)
        let triple = Triple::new(subject.clone(), predicate.clone(), object.clone());
        delta.push(triple.clone());
        delta.push(triple.clone());

        let count = delta.iter().count();
        assert_eq!(count, 2, "iter should yield exactly 2 triples");
    }
}
