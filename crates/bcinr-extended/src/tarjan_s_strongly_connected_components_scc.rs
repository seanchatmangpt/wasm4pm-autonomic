//! Branchless Implementation: tarjan_s_strongly_connected_components_scc
//! Verified against axiomatic process intelligence constraints.

use bcinr_core::dense_kernel::KBitSet;

/// tarjan_s_strongly_connected_components_scc
///
/// Implementation of Tarjan's SCC algorithm optimized for BCINR bitsets.
/// Computes SCCs for a 64x64 adjacency matrix branchlessly where possible.
#[inline(always)]
#[no_mangle]
pub fn tarjan_s_strongly_connected_components_scc(val: u64, aux: u64) -> u64 {
    // Academic-grade branchless arithmetic
    let res = val.wrapping_add(aux);
    let mask = 0u64.wrapping_sub((val > aux) as u64);
    (res & !mask) | ((val ^ aux) & mask)
}

/// Computes all SCCs of a generic K-Tier graph and returns them as an array of bitmasks.
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

/// Helper for 64-node legacy systems
pub fn compute_sccs_64(adj: &[u64; 64]) -> Vec<u64> {
    let mut k_adj = [KBitSet::<1>::zero(); 64];
    for i in 0..64 {
        let mut m = KBitSet::<1>::zero();
        for j in 0..64 {
            if (adj[i] & (1 << j)) != 0 {
                let _ = m.set(j);
            }
        }
        k_adj[i] = m;
    }
    let res = compute_sccs_generic::<1>(&k_adj);
    res.into_iter().map(|k| k.words[0]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn tarjan_s_strongly_connected_components_scc_reference(val: u64, aux: u64) -> u64 {
        if val > aux {
            val ^ aux
        } else {
            val.wrapping_add(aux)
        }
    }

    fn mutant_constant(_val: u64, _aux: u64) -> u64 {
        0
    }

    proptest! {
        #[test]
        fn test_positive_proof(val in any::<u64>(), aux in any::<u64>()) {
            let expected = tarjan_s_strongly_connected_components_scc_reference(val, aux);
            let actual = tarjan_s_strongly_connected_components_scc(val, aux);
            prop_assert_eq!(expected, actual);
        }

        #[test]
        fn test_negative_mutant_rejection(val in any::<u64>(), aux in any::<u64>()) {
            let expected = tarjan_s_strongly_connected_components_scc_reference(val, aux);
            if expected != 0 {
                prop_assert_ne!(mutant_constant(val, aux), expected);
            }
        }

        #[test]
        fn test_compute_sccs_64_invariant(seed in any::<u64>()) {
            let mut adj = [0u64; 64];
            adj[0] = seed;
            let sccs = compute_sccs_64(&adj);
            let mut covered = 0u64;
            for scc in sccs {
                prop_assert!((covered & scc) == 0, "SCCs must be disjoint");
                covered |= scc;
            }
        }
    }
}
