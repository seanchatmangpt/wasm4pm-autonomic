//! POWL64 geometric routing + BLAKE3 receipt chain.
//!
//! Sparse 64×64×64 cell universe addressed by GlobeCell-packed coordinates.
//! Each [`Powl64Cell`] carries a BLAKE3 source-IRI hash and links to its
//! prior chain hash to form a cryptographic chain through `Runtime::step`.
//!
//! # Geometry
//!
//! ```text
//! 64³ = 64 × 64 × 64 = 262,144 independent places
//!       ─┬─   ─┬─   ─┬─
//!        │     │     └── 64 places per attention cell
//!        │     └──────── 64 cells per domain
//!        └────────────── 64 domains in the globe
//! ```
//!
//! Storage is sparse: only cells touched by [`Powl64::extend`] are
//! materialized. A program with two extends occupies two `HashMap` entries,
//! never the full 262,144 slot grid. Coord collisions are
//! collision-preserving — multiple cells may share a coordinate (the
//! coordinate is a routing label, not a primary key).
//!
//! # Chain receipts
//!
//! Genesis: `chain_hash = blake3(source_hash || polarity)` (the polarity
//! byte is folded in even at genesis).
//! Subsequent: `chain_hash = blake3(prior_chain_hash || source_hash || polarity)`,
//! folded through a 65-byte stack buffer (no heap allocation per step).

use std::collections::HashMap;

use crate::graph::GraphIri;

// =============================================================================
// PACKING CONSTANTS
// =============================================================================

/// Bits per coordinate component (6 bits ⇒ 0..64).
pub const COORD_BITS: u32 = 6;

// =============================================================================
// COORD ERROR
// =============================================================================

/// Errors returned by checked [`GlobeCell`] construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordError {
    /// One of the coord components exceeded 6 bits (≥64).
    OutOfRange,
}

impl std::fmt::Display for CoordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange => f.write_str("coordinate component out of range (must be < 64)"),
        }
    }
}

impl std::error::Error for CoordError {}

// =============================================================================
// GLOBE CELL
// =============================================================================

/// Packed `(domain, cell, place)` coordinate — 6 bits per component, low 18
/// bits used.
///
/// Layout: `[63:18 unused][17:12 domain][11:6 cell][5:0 place]`. The `u64`
/// form is the canonical hash and `HashMap` key. Coordinate computation is
/// pure arithmetic: shifts and masks, no symbol table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct GlobeCell(pub u64);

impl GlobeCell {
    /// Mask for one 6-bit coordinate component (`0x3F`).
    pub const COORD_MASK: u64 = 0x3F;

    /// Bit offset of the `place` field inside the packed `u64`.
    pub const PLACE_SHIFT: u32 = 0;

    /// Bit offset of the `cell` field inside the packed `u64`.
    pub const CELL_SHIFT: u32 = 6;

    /// Bit offset of the `domain` field inside the packed `u64`.
    pub const DOMAIN_SHIFT: u32 = 12;

    /// The origin cell `(0, 0, 0)`.
    pub const ORIGIN: Self = Self(0);

    /// Construct from `(domain, cell, place)`, each in `0..64`.
    ///
    /// Values outside `0..64` are truncated to their low 6 bits. This is a
    /// const fn with no branches; callers on the hot path should validate
    /// inputs upstream. For a checked variant that rejects out-of-range
    /// inputs, see [`GlobeCell::try_new`].
    #[must_use]
    #[inline(always)]
    pub const fn new(domain: u8, cell: u8, place: u8) -> Self {
        let d = (domain as u64) & Self::COORD_MASK;
        let c = (cell as u64) & Self::COORD_MASK;
        let p = (place as u64) & Self::COORD_MASK;
        Self((d << Self::DOMAIN_SHIFT) | (c << Self::CELL_SHIFT) | (p << Self::PLACE_SHIFT))
    }

    /// Checked construction: returns [`CoordError::OutOfRange`] if any
    /// component is `≥64`.
    ///
    /// Use this on any path where coords come from external input. The
    /// branchless [`GlobeCell::new`] remains the fast path.
    ///
    /// # Errors
    ///
    /// Returns `Err(CoordError::OutOfRange)` when `domain >= 64`,
    /// `cell >= 64`, or `place >= 64`.
    pub fn try_new(domain: u8, cell: u8, place: u8) -> Result<Self, CoordError> {
        if domain >= 64 || cell >= 64 || place >= 64 {
            return Err(CoordError::OutOfRange);
        }
        Ok(Self::new(domain, cell, place))
    }

    /// Extract the domain component.
    #[must_use]
    #[inline(always)]
    pub const fn domain(self) -> u8 {
        ((self.0 >> Self::DOMAIN_SHIFT) & Self::COORD_MASK) as u8
    }

    /// Extract the cell component.
    #[must_use]
    #[inline(always)]
    pub const fn cell(self) -> u8 {
        ((self.0 >> Self::CELL_SHIFT) & Self::COORD_MASK) as u8
    }

    /// Extract the place component.
    #[must_use]
    #[inline(always)]
    pub const fn place(self) -> u8 {
        ((self.0 >> Self::PLACE_SHIFT) & Self::COORD_MASK) as u8
    }

    /// Derive a coordinate from the low 18 bits of a BLAKE3 hash.
    ///
    /// Uses the first 3 bytes of the hash in little-endian order, masked to
    /// 18 bits. The result is deterministic for a given hash, so equal
    /// receipts always land in the same cell.
    #[must_use]
    #[inline]
    pub fn from_hash_low18(h: &blake3::Hash) -> Self {
        let bytes = h.as_bytes();
        let raw = (bytes[0] as u64)
            | ((bytes[1] as u64) << 8)
            | ((bytes[2] as u64) << 16);
        Self(raw & 0x3_FFFF)
    }
}

// =============================================================================
// POWL64 CELL
// =============================================================================

/// Per-cell payload: source IRI hash, prior link, polarity, derived chain
/// hash.
///
/// `chain_hash` always folds polarity. At genesis it is
/// `blake3(source_hash || polarity)`. For every subsequent extend it is
/// `blake3(prior_chain_hash || source_hash || polarity)`.
#[derive(Clone, Debug)]
pub struct Powl64Cell {
    /// Packed coordinate where this cell lives in the 64³ universe.
    ///
    /// Note: multiple cells can share a coordinate after a low-18-bit hash
    /// collision (see [`Powl64::cells_at`]).
    pub coord: GlobeCell,
    /// Receipt polarity tag (caller-defined: e.g., `1 = required`).
    pub receipt_polarity: u8,
    /// BLAKE3 hash of the `breed_output_iri` string only.
    ///
    /// This is the hash of the IRI bytes — not a full semantic receipt.
    /// For a semantic receipt, see [`Powl64Cell::semantic_receipt`].
    pub source_hash: blake3::Hash,
    /// Deprecated alias for [`Powl64Cell::source_hash`]. Always equals
    /// `source_hash`.
    ///
    /// Retained as a public field for one release cycle so external
    /// callers using struct-field syntax keep compiling. New code must use
    /// [`Powl64Cell::source_hash`].
    #[deprecated(
        since = "0.1.0",
        note = "renamed to `source_hash` (it is only an IRI hash, not a full semantic receipt). \
                Use `source_hash` directly; this field will be removed."
    )]
    pub source_receipt: blake3::Hash,
    /// Optional full semantic receipt (e.g. derived from canonical material
    /// in `receipt.rs`). `None` when the cell was created via plain
    /// [`Powl64::extend`]; populated by
    /// [`Powl64::extend_with_semantic_receipt`].
    pub semantic_receipt: Option<blake3::Hash>,
    /// Prior chain hash if this cell extends an existing chain; `None` at
    /// genesis.
    pub prior_receipt: Option<blake3::Hash>,
    /// Derived chain hash that folds prior chain hash, source hash, and
    /// polarity. At genesis the prior is omitted.
    pub chain_hash: blake3::Hash,
}

// =============================================================================
// POWL64 UNIVERSE
// =============================================================================

/// Sparse Powl64 universe + chain head cursor + ordered replay path.
///
/// Holds only the cells produced by [`Powl64::extend`]. The chain is
/// linear in the current phase; full DAG fan-out is future work. Coord
/// collisions are preserved: each coordinate maps to a `Vec<Powl64Cell>`,
/// so two distinct chain hashes that hash to the same low-18-bit
/// coordinate both survive in storage.
#[derive(Debug, Default)]
pub struct Powl64 {
    /// Collision-preserving cell storage indexed by routing coordinate.
    cells: HashMap<GlobeCell, Vec<Powl64Cell>>,
    /// Coordinate of the most-recently inserted cell.
    cursor: GlobeCell,
    /// Most recent chain hash, or `None` before any extend.
    chain_head: Option<blake3::Hash>,
    /// Append-only ordered list of chain hashes — deterministic replay.
    path: Vec<blake3::Hash>,
    /// Length of the chain — equals `path.len()` and may exceed the count
    /// of distinct coordinates after collisions.
    chain_len: u64,
}

impl Powl64 {
    /// Build an empty universe with no cells and no chain head.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of cells stored across all coordinates (sums collision
    /// buckets).
    ///
    /// This is **not** the number of distinct coordinates — for that, use
    /// [`Powl64::coord_count`].
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.values().map(Vec::len).sum()
    }

    /// Number of distinct [`GlobeCell`] coordinates currently materialized.
    ///
    /// Differs from [`Powl64::cell_count`] when coord collisions occurred.
    #[must_use]
    pub fn coord_count(&self) -> usize {
        self.cells.len()
    }

    /// Length of the chain — independent of cell count under coord
    /// collisions. Equals `path().len()`.
    #[must_use]
    pub fn chain_len(&self) -> u64 {
        self.chain_len
    }

    /// Ordered replay path of chain hashes, in insertion order.
    ///
    /// This is the canonical material for v1 strong-equivalence shape
    /// matching. See [`Powl64::shape_match_v1_path`].
    #[must_use]
    pub fn path(&self) -> &[blake3::Hash] {
        &self.path
    }

    /// Current chain head, or `None` before any extend.
    #[must_use]
    pub fn chain_head(&self) -> Option<blake3::Hash> {
        self.chain_head
    }

    /// Coordinate of the most recently inserted cell (or origin if empty).
    #[must_use]
    pub fn cursor(&self) -> GlobeCell {
        self.cursor
    }

    /// Borrow the **first** cell at `coord`, if present.
    ///
    /// Retained for backward compatibility. After coord collisions multiple
    /// cells may share a coordinate; use [`Powl64::cells_at`] to access all
    /// of them.
    #[must_use]
    pub fn cell_at(&self, coord: GlobeCell) -> Option<&Powl64Cell> {
        self.cells.get(&coord).and_then(|v| v.first())
    }

    /// Borrow all cells at `coord` (multiple if coord collisions occurred).
    ///
    /// Returns an empty slice if no cell has been extended at this
    /// coordinate.
    #[must_use]
    pub fn cells_at(&self, coord: GlobeCell) -> &[Powl64Cell] {
        self.cells
            .get(&coord)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Compute the chain hash for the next extend given the current head,
    /// the new source hash, and polarity.
    #[inline]
    fn fold_chain(
        prior: Option<blake3::Hash>,
        source_hash: &blake3::Hash,
        polarity: u8,
    ) -> blake3::Hash {
        if let Some(prior) = prior {
            let mut buf = [0u8; 32 + 32 + 1];
            buf[..32].copy_from_slice(prior.as_bytes());
            buf[32..64].copy_from_slice(source_hash.as_bytes());
            buf[64] = polarity;
            blake3::hash(&buf)
        } else {
            // Genesis: hash source || polarity (still folds polarity in).
            let mut buf = [0u8; 32 + 1];
            buf[..32].copy_from_slice(source_hash.as_bytes());
            buf[32] = polarity;
            blake3::hash(&buf)
        }
    }

    /// Append a new cell to the chain.
    ///
    /// Hashes the IRI to produce `source_hash`. Folds prior chain head (if
    /// any) with `source_hash` and `polarity` through a stack buffer to
    /// derive `chain_hash`. Coordinate is the low-18-bit projection of
    /// `chain_hash`. Updates cursor, chain head, and ordered replay path,
    /// then returns a clone of the new cell.
    ///
    /// `semantic_receipt` is set to `None`; use
    /// [`Powl64::extend_with_semantic_receipt`] to populate it.
    ///
    /// # Collision
    ///
    /// If two distinct chain hashes happen to project to the same 18-bit
    /// coordinate, both cells survive in `cells_at(coord)`. The chain hash
    /// itself remains globally distinct; only the geometric address aliases.
    pub fn extend(&mut self, breed_output_iri: &GraphIri, polarity: u8) -> Powl64Cell {
        self.extend_inner(breed_output_iri, polarity, None)
    }

    /// Append a new cell carrying a precomputed semantic receipt.
    ///
    /// Behaves exactly like [`Powl64::extend`] except that the resulting
    /// cell stores `Some(semantic_receipt)`. The semantic receipt is **not**
    /// folded into `chain_hash`; the chain folds only `prior || source_hash
    /// || polarity` so existing receipt-chain semantics are preserved.
    pub fn extend_with_semantic_receipt(
        &mut self,
        iri: &GraphIri,
        polarity: u8,
        semantic_receipt: blake3::Hash,
    ) -> Powl64Cell {
        self.extend_inner(iri, polarity, Some(semantic_receipt))
    }

    fn extend_inner(
        &mut self,
        breed_output_iri: &GraphIri,
        polarity: u8,
        semantic_receipt: Option<blake3::Hash>,
    ) -> Powl64Cell {
        let source_hash = blake3::hash(breed_output_iri.as_str().as_bytes());
        let chain_hash = Self::fold_chain(self.chain_head, &source_hash, polarity);
        let coord = GlobeCell::from_hash_low18(&chain_hash);
        #[allow(deprecated)]
        let cell = Powl64Cell {
            coord,
            receipt_polarity: polarity,
            source_hash,
            // Deprecated alias mirrors `source_hash` for one release cycle.
            source_receipt: source_hash,
            semantic_receipt,
            prior_receipt: self.chain_head,
            chain_hash,
        };
        self.cells.entry(coord).or_default().push(cell.clone());
        self.path.push(chain_hash);
        self.cursor = coord;
        self.chain_head = Some(chain_hash);
        self.chain_len += 1;
        cell
    }

    /// Shape match v0 — cell-count equivalence.
    ///
    /// Two universes are v0-equivalent iff they have the same materialized
    /// cell count (sum across coord buckets, including collisions). This is
    /// the weakest meaningful invariant — it confirms two chains span the
    /// same number of extends, not that they followed the same path. For
    /// ordered-replay strong equivalence see [`Powl64::shape_match_v1_path`].
    ///
    /// # Errors
    ///
    /// Returns `Err(message)` describing the cell-count mismatch.
    pub fn shape_match_v0_cell_count(&self, other: &Powl64) -> Result<(), String> {
        let a = self.cell_count();
        let b = other.cell_count();
        if a == b {
            Ok(())
        } else {
            Err(format!("cell count mismatch: {} vs {}", a, b))
        }
    }

    /// Deprecated alias for [`Powl64::shape_match_v0_cell_count`].
    ///
    /// Retained for one release cycle so external callers can migrate.
    ///
    /// # Errors
    ///
    /// Same as [`Powl64::shape_match_v0_cell_count`].
    #[deprecated(
        since = "0.1.0",
        note = "renamed to `shape_match_v0_cell_count` (cell-count equivalence). \
                For ordered-path equivalence use `shape_match_v1_path`."
    )]
    pub fn shape_match(&self, other: &Powl64) -> Result<(), String> {
        self.shape_match_v0_cell_count(other)
    }

    /// Strong-equivalence shape match: ordered replay-path comparison.
    ///
    /// Returns `Ok(())` iff both universes have the same chain length and
    /// every position of the [`path`](Powl64::path) matches byte-for-byte.
    /// This is the v1 invariant — v0 cell-count parity is too weak under
    /// coord collisions.
    ///
    /// # Errors
    ///
    /// Returns `Err(diagnostic)` describing the first divergence (length
    /// mismatch, or the index at which the chain hashes diverged).
    pub fn shape_match_v1_path(&self, other: &Powl64) -> Result<(), String> {
        if self.path.len() != other.path.len() {
            return Err(format!(
                "path length mismatch: {} vs {}",
                self.path.len(),
                other.path.len()
            ));
        }
        for (i, (a, b)) in self.path.iter().zip(other.path.iter()).enumerate() {
            if a.as_bytes() != b.as_bytes() {
                return Err(format!(
                    "path divergence at index {}: {} vs {}",
                    i,
                    a.to_hex(),
                    b.to_hex()
                ));
            }
        }
        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globe_cell_packs_and_unpacks() {
        let g = GlobeCell::new(7, 13, 42);
        assert_eq!(g.domain(), 7);
        assert_eq!(g.cell(), 13);
        assert_eq!(g.place(), 42);
    }

    #[test]
    fn globe_cell_origin_is_zero() {
        assert_eq!(GlobeCell::ORIGIN.0, 0);
        assert_eq!(GlobeCell::ORIGIN.domain(), 0);
        assert_eq!(GlobeCell::ORIGIN.cell(), 0);
        assert_eq!(GlobeCell::ORIGIN.place(), 0);
    }

    #[test]
    fn globe_cell_truncates_oversized_inputs() {
        // 64 wraps to 0 since only low 6 bits are kept.
        let g = GlobeCell::new(64, 64, 64);
        assert_eq!(g.0, 0);
    }

    #[test]
    fn from_hash_low18_uses_first_three_bytes_le() {
        let h = blake3::hash(b"deterministic input");
        let g = GlobeCell::from_hash_low18(&h);
        let bytes = h.as_bytes();
        let expected = ((bytes[0] as u64)
            | ((bytes[1] as u64) << 8)
            | ((bytes[2] as u64) << 16))
            & 0x3_FFFF;
        assert_eq!(g.0, expected);
    }

    #[test]
    fn empty_universe_has_no_chain_head() {
        let p = Powl64::new();
        assert_eq!(p.cell_count(), 0);
        assert_eq!(p.coord_count(), 0);
        assert_eq!(p.chain_len(), 0);
        assert!(p.chain_head().is_none());
        assert_eq!(p.cursor(), GlobeCell::ORIGIN);
        assert!(p.path().is_empty());
    }

    #[test]
    fn shape_match_succeeds_for_equal_cell_counts() {
        let a = Powl64::new();
        let b = Powl64::new();
        assert!(a.shape_match_v0_cell_count(&b).is_ok());
    }

    #[test]
    fn shape_match_fails_for_mismatched_counts() {
        let mut a = Powl64::new();
        let b = Powl64::new();
        let iri = GraphIri::from_iri("http://example.org/x").unwrap();
        a.extend(&iri, 1);
        assert!(a.shape_match_v0_cell_count(&b).is_err());
    }

    #[test]
    fn extend_appends_to_path() {
        let mut p = Powl64::new();
        let iri1 = GraphIri::from_iri("http://example.org/a").unwrap();
        let iri2 = GraphIri::from_iri("http://example.org/b").unwrap();
        let iri3 = GraphIri::from_iri("http://example.org/c").unwrap();
        p.extend(&iri1, 1);
        p.extend(&iri2, 1);
        p.extend(&iri3, 1);
        assert_eq!(p.path().len(), 3);
        assert_eq!(p.chain_len(), 3);
    }

    #[test]
    fn chain_hash_folds_polarity() {
        // Same iri sequence with a divergent polarity at step 2 must
        // produce different chain heads — proving polarity is folded into
        // the chain after genesis.
        let iri1 = GraphIri::from_iri("http://example.org/a").unwrap();
        let iri2 = GraphIri::from_iri("http://example.org/b").unwrap();

        let mut p1 = Powl64::new();
        p1.extend(&iri1, 1);
        let cell1 = p1.extend(&iri2, 1);

        let mut p2 = Powl64::new();
        p2.extend(&iri1, 1);
        let cell2 = p2.extend(&iri2, 2);

        assert_ne!(
            cell1.chain_hash.as_bytes(),
            cell2.chain_hash.as_bytes(),
            "polarity must affect chain_hash on non-genesis extends"
        );
    }

    #[test]
    fn genesis_uses_polarity() {
        // Genesis with the same iri but different polarities must produce
        // different chain hashes — proving polarity is folded in at the
        // genesis step too.
        let iri = GraphIri::from_iri("http://example.org/a").unwrap();

        let mut p1 = Powl64::new();
        let cell1 = p1.extend(&iri, 1);

        let mut p2 = Powl64::new();
        let cell2 = p2.extend(&iri, 2);

        assert_ne!(
            cell1.chain_hash.as_bytes(),
            cell2.chain_hash.as_bytes(),
            "genesis polarity must affect chain_hash"
        );
    }

    #[test]
    fn coord_collision_preserves_cells() {
        // We can't easily force two distinct chain hashes to project to
        // the same 18-bit coordinate without solving a hash preimage, so
        // we directly synthesize the collision by pushing two cells at the
        // same coord through the internal map and verify `cells_at`
        // returns both.
        let mut p = Powl64::new();
        let iri = GraphIri::from_iri("http://example.org/a").unwrap();
        let cell = p.extend(&iri, 1);
        let coord = cell.coord;

        // Manually inject a second cell at the same coord (synthetic
        // collision). The `cells` field is private to this module so this
        // uses internal access available only inside `tests`.
        let synthetic_source = blake3::hash(b"synthetic");
        #[allow(deprecated)]
        let synthetic = Powl64Cell {
            coord,
            receipt_polarity: 7,
            source_hash: synthetic_source,
            source_receipt: synthetic_source,
            semantic_receipt: None,
            prior_receipt: Some(cell.chain_hash),
            chain_hash: blake3::hash(b"synthetic-chain"),
        };
        p.cells.entry(coord).or_default().push(synthetic);

        let bucket = p.cells_at(coord);
        assert_eq!(bucket.len(), 2, "both cells survive the coord collision");
        assert_eq!(bucket[0].receipt_polarity, 1);
        assert_eq!(bucket[1].receipt_polarity, 7);
        // cell_count sums collision buckets.
        assert_eq!(p.cell_count(), 2);
        // coord_count counts distinct coords (only one here).
        assert_eq!(p.coord_count(), 1);
        // cell_at returns the FIRST cell at coord for backward compat.
        assert_eq!(
            p.cell_at(coord).expect("cell present").receipt_polarity,
            1
        );
    }

    #[test]
    fn cells_at_returns_empty_for_unknown_coord() {
        let p = Powl64::new();
        assert!(p.cells_at(GlobeCell::new(63, 63, 63)).is_empty());
    }

    #[test]
    fn path_strong_match_succeeds_for_identical_extends() {
        let iri1 = GraphIri::from_iri("http://example.org/a").unwrap();
        let iri2 = GraphIri::from_iri("http://example.org/b").unwrap();

        let mut p_a = Powl64::new();
        p_a.extend(&iri1, 1);
        p_a.extend(&iri2, 2);

        let mut p_b = Powl64::new();
        p_b.extend(&iri1, 1);
        p_b.extend(&iri2, 2);

        assert!(p_a.shape_match_v1_path(&p_b).is_ok());
    }

    #[test]
    fn path_strong_match_fails_for_divergent_chains() {
        let iri1 = GraphIri::from_iri("http://example.org/a").unwrap();
        let iri2 = GraphIri::from_iri("http://example.org/b").unwrap();
        let iri3 = GraphIri::from_iri("http://example.org/c").unwrap();

        let mut p_a = Powl64::new();
        p_a.extend(&iri1, 1);
        p_a.extend(&iri2, 1);

        let mut p_b = Powl64::new();
        p_b.extend(&iri1, 1);
        p_b.extend(&iri3, 1);

        assert!(p_a.shape_match_v1_path(&p_b).is_err());
    }

    #[test]
    fn try_new_rejects_oob_components() {
        assert_eq!(
            GlobeCell::try_new(64, 0, 0),
            Err(CoordError::OutOfRange)
        );
        assert_eq!(
            GlobeCell::try_new(0, 64, 0),
            Err(CoordError::OutOfRange)
        );
        assert_eq!(
            GlobeCell::try_new(0, 0, 64),
            Err(CoordError::OutOfRange)
        );
        assert_eq!(
            GlobeCell::try_new(255, 255, 255),
            Err(CoordError::OutOfRange)
        );
    }

    #[test]
    fn try_new_accepts_valid() {
        let g = GlobeCell::try_new(63, 63, 63).expect("63 is in range");
        assert_eq!(g.domain(), 63);
        assert_eq!(g.cell(), 63);
        assert_eq!(g.place(), 63);

        let zero = GlobeCell::try_new(0, 0, 0).expect("zero is valid");
        assert_eq!(zero, GlobeCell::ORIGIN);
    }

    #[test]
    fn deprecated_source_receipt_alias_returns_source_hash() {
        let iri = GraphIri::from_iri("http://example.org/a").unwrap();
        let mut p = Powl64::new();
        let cell = p.extend(&iri, 1);
        #[allow(deprecated)]
        let alias = cell.source_receipt;
        assert_eq!(alias.as_bytes(), cell.source_hash.as_bytes());
    }

    #[test]
    fn deprecated_shape_match_alias_matches_v0() {
        let mut a = Powl64::new();
        let mut b = Powl64::new();
        let iri = GraphIri::from_iri("http://example.org/a").unwrap();
        a.extend(&iri, 1);
        b.extend(&iri, 1);
        #[allow(deprecated)]
        let result = a.shape_match(&b);
        assert!(result.is_ok());
    }

    #[test]
    fn extend_with_semantic_receipt_populates_field() {
        let iri = GraphIri::from_iri("http://example.org/a").unwrap();
        let semantic = blake3::hash(b"semantic-payload");
        let mut p = Powl64::new();
        let cell = p.extend_with_semantic_receipt(&iri, 1, semantic);
        assert_eq!(
            cell.semantic_receipt.expect("semantic receipt populated").as_bytes(),
            semantic.as_bytes()
        );
        // Plain extend leaves it as None.
        let plain = p.extend(&iri, 1);
        assert!(plain.semantic_receipt.is_none());
    }
}
