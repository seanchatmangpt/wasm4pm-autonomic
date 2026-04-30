//! Postcard binary serialization for POWL8 / POWL64 (Phase 10, offline only).
//!
//! Postcard is canonical (deterministic byte layout) and `forbid(unsafe_code)`
//! compatible — the two reasons it was chosen over bincode (non-canonical)
//! and rkyv (requires `unsafe`).
//!
//! `Powl8Node`'s `BinaryRelation` carries `[u64; MAX_NODES]` (64 entries)
//! which exceeds serde's built-in array limit of 32, so we hand-roll a
//! lossless wire form via [`Powl8Wire`] / [`Powl8NodeWire`]. The wire form
//! is stable across crate releases — adding new node variants must only
//! append discriminants, never reuse them.

use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

use crate::powl::{BinaryRelation, Powl8, Powl8Node, MAX_NODES};
use crate::powl64::Powl64;
use crate::verdict::Breed;

/// Wire-form `BinaryRelation` — explicit length-prefixed `Vec<u64>` to avoid
/// serde's 32-entry array cap.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BinaryRelationWire {
    /// 64 row words (`[u64; 64]` flattened to a `Vec<u64>`).
    pub words: Vec<u64>,
}

impl From<BinaryRelation> for BinaryRelationWire {
    fn from(b: BinaryRelation) -> Self {
        let mut words = Vec::with_capacity(MAX_NODES);
        for i in 0..MAX_NODES {
            // Read each row via `is_edge` reconstructed bits — but the
            // public surface only exposes `is_edge`, so we rebuild row-wise.
            let mut row = 0u64;
            for j in 0..MAX_NODES {
                if b.is_edge(i, j) {
                    row |= 1u64 << j;
                }
            }
            words.push(row);
        }
        Self { words }
    }
}

impl From<BinaryRelationWire> for BinaryRelation {
    fn from(w: BinaryRelationWire) -> Self {
        let mut r = BinaryRelation::new();
        for (i, row) in w.words.iter().enumerate().take(MAX_NODES) {
            let mut bits = *row;
            while bits != 0 {
                let j = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                if j < MAX_NODES {
                    r.add_edge(i, j);
                }
            }
        }
        r
    }
}

/// Wire-form `Powl8Node` (Phase-10 stable ABI). New variants append; never
/// reuse a discriminant.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Powl8NodeWire {
    /// Silent runtime marker.
    Silent,
    /// Activity referencing a breed (`u8` discriminant of `Breed`).
    Activity {
        /// Breed discriminant (`Breed as u8`).
        breed: u8,
    },
    /// Sub-plan with explicit partial order.
    PartialOrder {
        /// First child global index.
        start: u16,
        /// Number of children.
        count: u16,
        /// Bit-matrix.
        rel: BinaryRelationWire,
    },
    /// Sequence operator.
    OperatorSequence {
        /// Predecessor index.
        a: u16,
        /// Successor index.
        b: u16,
    },
    /// Parallel operator.
    OperatorParallel {
        /// First parallel child.
        a: u16,
        /// Second parallel child.
        b: u16,
    },
    /// Plan entry marker.
    StartNode,
    /// Plan exit marker.
    EndNode,
    /// Phase-10 Choice: at most four branch indices.
    Choice {
        /// Branch indices (length 4 vec, only first `len` valid).
        branches: Vec<u16>,
        /// Live branch count.
        len: u8,
    },
    /// Phase-10 bounded Loop.
    Loop {
        /// Body node index.
        body: u16,
        /// Maximum iterations (≤ 16).
        max_iters: u8,
    },
}

impl From<Powl8Node> for Powl8NodeWire {
    fn from(n: Powl8Node) -> Self {
        match n {
            Powl8Node::Silent => Self::Silent,
            Powl8Node::Activity(b) => Self::Activity { breed: b as u8 },
            Powl8Node::PartialOrder { start, count, rel } => Self::PartialOrder {
                start,
                count,
                rel: rel.into(),
            },
            Powl8Node::OperatorSequence { a, b } => Self::OperatorSequence { a, b },
            Powl8Node::OperatorParallel { a, b } => Self::OperatorParallel { a, b },
            Powl8Node::StartNode => Self::StartNode,
            Powl8Node::EndNode => Self::EndNode,
            Powl8Node::Choice { branches, len } => Self::Choice {
                branches: branches.to_vec(),
                len,
            },
            Powl8Node::Loop { body, max_iters } => Self::Loop { body, max_iters },
        }
    }
}

impl TryFrom<Powl8NodeWire> for Powl8Node {
    type Error = postcard::Error;
    fn try_from(w: Powl8NodeWire) -> Result<Self, Self::Error> {
        Ok(match w {
            Powl8NodeWire::Silent => Powl8Node::Silent,
            Powl8NodeWire::Activity { breed } => Powl8Node::Activity(breed_from_u8(breed)?),
            Powl8NodeWire::PartialOrder { start, count, rel } => Powl8Node::PartialOrder {
                start,
                count,
                rel: rel.into(),
            },
            Powl8NodeWire::OperatorSequence { a, b } => Powl8Node::OperatorSequence { a, b },
            Powl8NodeWire::OperatorParallel { a, b } => Powl8Node::OperatorParallel { a, b },
            Powl8NodeWire::StartNode => Powl8Node::StartNode,
            Powl8NodeWire::EndNode => Powl8Node::EndNode,
            Powl8NodeWire::Choice { branches, len } => {
                let mut out = [0u16; 4];
                for (i, v) in branches.iter().take(4).enumerate() {
                    out[i] = *v;
                }
                Powl8Node::Choice { branches: out, len }
            }
            Powl8NodeWire::Loop { body, max_iters } => Powl8Node::Loop { body, max_iters },
        })
    }
}

fn breed_from_u8(b: u8) -> Result<Breed, postcard::Error> {
    Ok(match b {
        0 => Breed::Eliza,
        1 => Breed::Mycin,
        2 => Breed::Strips,
        3 => Breed::Shrdlu,
        4 => Breed::Prolog,
        5 => Breed::Hearsay,
        6 => Breed::Dendral,
        7 => Breed::CompiledHook,
        8 => Breed::Gps,
        9 => Breed::Soar,
        10 => Breed::Prs,
        11 => Breed::Cbr,
        _ => return Err(postcard::Error::SerdeDeCustom),
    })
}

/// Wire-form `Powl8`: deterministic vec of nodes + root.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Powl8Wire {
    /// Stable-ABI node sequence.
    pub nodes: Vec<Powl8NodeWire>,
    /// Root index into `nodes`.
    pub root: u16,
}

impl From<&Powl8> for Powl8Wire {
    fn from(p: &Powl8) -> Self {
        Self {
            nodes: p.nodes.iter().copied().map(Powl8NodeWire::from).collect(),
            root: p.root,
        }
    }
}

impl TryFrom<Powl8Wire> for Powl8 {
    type Error = postcard::Error;
    fn try_from(w: Powl8Wire) -> Result<Self, Self::Error> {
        let mut nodes = Vec::with_capacity(w.nodes.len());
        for n in w.nodes {
            nodes.push(Powl8Node::try_from(n)?);
        }
        Ok(Powl8 {
            nodes,
            root: w.root,
        })
    }
}

/// Encode a [`Powl8`] to canonical postcard bytes (offline only).
pub fn powl8_to_postcard(plan: &Powl8) -> Result<Vec<u8>, postcard::Error> {
    let wire: Powl8Wire = plan.into();
    to_allocvec(&wire)
}

/// Decode a [`Powl8`] from canonical postcard bytes (offline only).
pub fn powl8_from_postcard(bytes: &[u8]) -> Result<Powl8, postcard::Error> {
    let wire: Powl8Wire = from_bytes(bytes)?;
    Powl8::try_from(wire)
}

// =============================================================================
// POWL64
// =============================================================================

/// Wire-form Powl64 path: ordered chain-hash bytes (the canonical replay
/// record). The full cell map is not part of the offline ABI — it is
/// reconstructible by replaying the path through a fresh `Powl64` against
/// the same source IRI sequence.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Powl64PathWire {
    /// Chain-hash bytes in extension order, 32 bytes per entry.
    pub chain_hashes: Vec<[u8; 32]>,
}

/// Encode a `Powl64` path (chain-hash sequence) to postcard. Offline only.
///
/// The chain-hash sequence is the canonical replay record: each entry folds
/// polarity at every step (genesis included), so two distinct producers
/// that yield the same chain head must have produced byte-identical paths.
pub fn powl64_to_postcard(p: &Powl64) -> Result<Vec<u8>, postcard::Error> {
    let chain_hashes: Vec<[u8; 32]> = p.path().iter().map(|h| *h.as_bytes()).collect();
    let wire = Powl64PathWire { chain_hashes };
    to_allocvec(&wire)
}

/// Decode a `Powl64` chain-hash path from postcard (offline only). Returns
/// a vec of chain hashes; a fresh `Powl64` can replay these against an IRI
/// sequence to reconstruct the live universe.
pub fn powl64_from_postcard(bytes: &[u8]) -> Result<Vec<blake3::Hash>, postcard::Error> {
    let wire: Powl64PathWire = from_bytes(bytes)?;
    Ok(wire
        .chain_hashes
        .into_iter()
        .map(blake3::Hash::from_bytes)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::powl::Powl8Node;

    #[test]
    fn powl8_roundtrip_through_postcard_preserves_choice_loop() {
        let mut p = Powl8::new();
        p.push(Powl8Node::StartNode).unwrap();
        p.push(Powl8Node::Activity(Breed::Gps)).unwrap();
        p.push(Powl8Node::Activity(Breed::Cbr)).unwrap();
        p.push(Powl8Node::Choice {
            branches: [1, 2, 0, 0],
            len: 2,
        })
        .unwrap();
        p.push(Powl8Node::Loop {
            body: 1,
            max_iters: 4,
        })
        .unwrap();
        p.push(Powl8Node::EndNode).unwrap();
        let bytes = powl8_to_postcard(&p).unwrap();
        let p2 = powl8_from_postcard(&bytes).unwrap();
        assert_eq!(p2.nodes.len(), p.nodes.len());
        // Spot-check the Choice and Loop survived.
        assert!(matches!(p2.nodes[3], Powl8Node::Choice { len: 2, .. }));
        assert!(matches!(
            p2.nodes[4],
            Powl8Node::Loop {
                body: 1,
                max_iters: 4
            }
        ));
    }
}
