//! POWL64 BLAKE3 receipt-chain unit tests.
//!
//! Covers:
//! - Genesis: first extend has no prior, chain_hash == source_receipt.
//! - Chain: second extend's prior_receipt == first cell's chain_hash, and
//!   second cell's chain_hash differs from its source_receipt.
//! - Determinism: identical IRI sequences produce identical chain heads.
//! - Sparse storage: cell_count is the extend count, never 64³.

use ccog::graph::GraphIri;
use ccog::powl64::Powl64;

#[test]
fn genesis_extend_has_no_prior_and_polarity_folded_chain() {
    // Phase 5 Track C: genesis chain hash now folds polarity into
    // `blake3(source_hash || polarity)`, so it differs from source_hash.
    let iri = GraphIri::from_iri("http://example.org/breed/eliza/output/1").unwrap();
    let mut p = Powl64::new();

    let cell = p.extend(&iri, 1);

    assert!(
        cell.prior_receipt.is_none(),
        "genesis cell must have no prior receipt"
    );
    assert_ne!(
        cell.chain_hash,
        cell.source_hash,
        "genesis chain_hash now folds polarity → must differ from source_hash"
    );
    assert_eq!(
        p.chain_head(),
        Some(cell.chain_hash),
        "chain head tracks the genesis cell's chain_hash"
    );
    assert_eq!(p.cell_count(), 1, "exactly one cell after one extend");
    assert_eq!(p.cursor(), cell.coord, "cursor points at the genesis cell");
    assert_eq!(cell.receipt_polarity, 1, "polarity passed through");
}

#[test]
fn chain_links_prior_receipt_into_subsequent_chain_hash() {
    let iri1 = GraphIri::from_iri("http://example.org/breed/strips/output/1").unwrap();
    let iri2 = GraphIri::from_iri("http://example.org/breed/mycin/output/2").unwrap();
    let mut p = Powl64::new();

    let cell1 = p.extend(&iri1, 1);
    let cell2 = p.extend(&iri2, 1);

    assert_eq!(
        cell2.prior_receipt,
        Some(cell1.chain_hash),
        "second cell must reference first cell's chain_hash as prior"
    );
    assert_ne!(
        cell2.chain_hash, cell2.source_receipt,
        "second cell's chain_hash must differ from its source_receipt (prior was folded in)"
    );
    assert_ne!(
        cell1.chain_hash, cell2.chain_hash,
        "two distinct extends must produce distinct chain hashes"
    );
    assert_eq!(
        p.chain_head(),
        Some(cell2.chain_hash),
        "chain head advances to the latest cell"
    );
    assert_eq!(p.cell_count(), 2);
}

#[test]
fn determinism_same_iri_sequence_yields_same_chain_head() {
    let iri1 = GraphIri::from_iri("http://example.org/breed/dendral/output/a").unwrap();
    let iri2 = GraphIri::from_iri("http://example.org/breed/hearsay/output/b").unwrap();
    let iri3 = GraphIri::from_iri("http://example.org/breed/prolog/output/c").unwrap();

    let mut p_a = Powl64::new();
    p_a.extend(&iri1, 1);
    p_a.extend(&iri2, 1);
    p_a.extend(&iri3, 1);

    let mut p_b = Powl64::new();
    p_b.extend(&iri1, 1);
    p_b.extend(&iri2, 1);
    p_b.extend(&iri3, 1);

    assert_eq!(
        p_a.chain_head(),
        p_b.chain_head(),
        "identical IRI sequences must produce identical chain heads"
    );
    assert_eq!(
        p_a.cell_count(),
        p_b.cell_count(),
        "identical IRI sequences must produce identical cell counts"
    );
    assert!(
        p_a.shape_match(&p_b).is_ok(),
        "shape_match must succeed on equal-cell-count universes"
    );
}

#[test]
fn sparse_storage_two_extends_two_cells_not_262144() {
    let iri1 = GraphIri::from_iri("http://example.org/breed/eliza/x").unwrap();
    let iri2 = GraphIri::from_iri("http://example.org/breed/eliza/y").unwrap();
    let mut p = Powl64::new();

    p.extend(&iri1, 1);
    p.extend(&iri2, 1);

    assert_eq!(
        p.cell_count(),
        2,
        "sparse universe stores only what was extended (NOT 64³ = 262,144)"
    );
}
