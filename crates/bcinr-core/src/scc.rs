use crate::dense_kernel::KBitSet;

/// Computes all SCCs of a generic K-Tier graph and returns them as an array of bitmasks.
/// Implementation based on transitive closure intersections.
#[allow(clippy::needless_range_loop)]
pub fn compute_sccs_generic<const WORDS: usize>(adj: &[KBitSet<WORDS>]) -> Vec<KBitSet<WORDS>> {
    let max_nodes = WORDS * 64;
    let mut sccs = Vec::new();
    let mut visited = KBitSet::<WORDS>::zero();

    let mut r = adj.to_vec();
    // Transitive Closure (Branchless)
    for k in 0..max_nodes {
        for i in 0..max_nodes {
            if r[i].contains(k) {
                let k_mask = r[k];
                r[i] = r[i].bitwise_or(k_mask);
            }
        }
    }

    // Transpose Reachability to get Column masks
    let mut rt = vec![KBitSet::<WORDS>::zero(); max_nodes];
    for i in 0..max_nodes {
        for j in 0..max_nodes {
            if r[i].contains(j) {
                let _ = rt[j].set(i);
            }
        }
    }

    for i in 0..max_nodes {
        if !visited.contains(i) {
            // SCC for node i is nodes reachable from i AND nodes that can reach i
            let mut scc = r[i].bitwise_and(rt[i]);
            let _ = scc.set(i);
            sccs.push(scc);
            visited = visited.bitwise_or(scc);
        }
    }

    sccs
}
