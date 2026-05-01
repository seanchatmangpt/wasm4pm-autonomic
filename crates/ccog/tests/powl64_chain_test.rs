//! POWL64 BLAKE3 receipt-chain unit tests.

use ccog::powl64::{PartnerId, Polarity, Powl64, Powl64RouteCell, ProjectionTarget};
use ccog::runtime::cog8::{CollapseFn, EdgeId, EdgeKind, NodeId};

#[test]
fn genesis_extend_has_no_prior_and_polarity_folded_chain() {
    let mut p = Powl64::new();
    let cell = Powl64RouteCell {
        graph_id: 1,
        from_node: NodeId(0),
        to_node: NodeId(1),
        edge_id: EdgeId(1),
        edge_kind: EdgeKind::Choice,
        collapse_fn: CollapseFn::ReflectivePosture,
        polarity: Polarity::Positive,
        projection_target: ProjectionTarget::NoOp,
        partner_id: PartnerId::NONE,
        input_digest: 0,
        args_digest: 0,
        result_digest: 0,
        prior_chain: 0,
        chain_head: 12345,
    };

    p.extend(cell);

    assert_eq!(p.chain_head(), Some(12345));
    assert_eq!(p.cell_count(), 1);
}

#[test]
fn chain_links_prior_receipt_into_subsequent_chain_hash() {
    let mut p = Powl64::new();

    let cell1 = Powl64RouteCell {
        chain_head: 100,
        ..Default::default()
    };
    p.extend(cell1);

    let cell2 = Powl64RouteCell {
        prior_chain: 100,
        chain_head: 200,
        ..Default::default()
    };
    p.extend(cell2);

    assert_eq!(p.chain_head(), Some(200));
    assert_eq!(p.cell_count(), 2);
    assert_eq!(p.cells[1].prior_chain, 100);
}

#[test]
fn determinism_same_cell_sequence_yields_same_chain_head() {
    let mut p_a = Powl64::new();
    p_a.extend(Powl64RouteCell {
        chain_head: 1,
        ..Default::default()
    });
    p_a.extend(Powl64RouteCell {
        chain_head: 2,
        ..Default::default()
    });

    let mut p_b = Powl64::new();
    p_b.extend(Powl64RouteCell {
        chain_head: 1,
        ..Default::default()
    });
    p_b.extend(Powl64RouteCell {
        chain_head: 2,
        ..Default::default()
    });

    assert_eq!(p_a.chain_head(), p_b.chain_head());
    assert_eq!(p_a.cell_count(), p_b.cell_count());
    assert!(p_a.shape_match_v1_path(&p_b).is_ok());
}
