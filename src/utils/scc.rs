use crate::utils::dense_kernel::KBitSet;
use std::cmp::min;

/// Computes all SCCs of a generic K-Tier graph using Tarjan's $O(V+E)$ algorithm.
/// Optimized for sparse Directly Follows Graphs (DFG).
#[allow(clippy::needless_range_loop)]
pub fn compute_sccs_generic<const WORDS: usize>(adj: &[KBitSet<WORDS>]) -> Vec<KBitSet<WORDS>> {
    let max_nodes = WORDS * 64;
    let mut sccs = Vec::new();
    
    let mut index = 0;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; max_nodes];
    let mut indices = vec![-1; max_nodes];
    let mut lowlink = vec![-1; max_nodes];

    fn strong_connect<const W: usize>(
        v: usize,
        adj: &[KBitSet<W>],
        index: &mut i32,
        stack: &mut Vec<usize>,
        on_stack: &mut [bool],
        indices: &mut [i32],
        lowlink: &mut [i32],
        sccs: &mut Vec<KBitSet<W>>,
        max_nodes: usize,
    ) {
        indices[v] = *index;
        lowlink[v] = *index;
        *index += 1;
        stack.push(v);
        on_stack[v] = true;

        // Explore neighbors
        for w in 0..max_nodes {
            if adj[v].contains(w) {
                if indices[w] == -1 {
                    strong_connect(w, adj, index, stack, on_stack, indices, lowlink, sccs, max_nodes);
                    lowlink[v] = min(lowlink[v], lowlink[w]);
                } else if on_stack[w] {
                    lowlink[v] = min(lowlink[v], indices[w]);
                }
            }
        }

        // If v is a root node, pop the stack and generate an SCC
        if lowlink[v] == indices[v] {
            let mut scc_mask = KBitSet::<W>::zero();
            loop {
                let w = stack.pop().unwrap();
                on_stack[w] = false;
                let _ = scc_mask.set(w);
                if w == v { break; }
            }
            sccs.push(scc_mask);
        }
    }

    for i in 0..max_nodes {
        // Only start from nodes that have at least one edge or are part of the footprint
        // The caller in powl/discovery.rs already filters tdfg, but we double check
        if indices[i] == -1 && (!adj[i].is_empty() || (0..max_nodes).any(|prev| adj[prev].contains(i))) {
            strong_connect(i, adj, &mut index, &mut stack, &mut on_stack, &mut indices, &mut lowlink, &mut sccs, max_nodes);
        }
    }

    sccs
}
