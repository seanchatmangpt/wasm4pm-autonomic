use crate::powl::core::{PowlNode, PowlOperator};
use bcinr_core::dense_kernel::KBitSet;

/// High-performance Nanosecond Inductive Miner over BCINR Bitsets
/// Partitions a Directly Follows Graph (DFG) into a hierarchical POWL AST.
#[allow(clippy::needless_range_loop)]
pub fn mine_powl<const WORDS: usize>(
    dfg: &[KBitSet<WORDS>],
    footprint: KBitSet<WORDS>,
    activity_names: &[String],
) -> PowlNode {
    if footprint.is_empty() {
        return PowlNode::dummy();
    }

    // Count ones in footprint
    let mut count = 0;
    let mut last_bit = 0;
    for i in 0..(WORDS * 64) {
        if footprint.contains(i) {
            count += 1;
            last_bit = i;
        }
    }

    if count == 1 {
        let label = activity_names
            .get(last_bit)
            .cloned()
            .unwrap_or_else(|| format!("Activity_{}", last_bit));
        return PowlNode::Transition {
            label: Some(label),
            id: last_bit as u64,
        };
    }

    // 1. XOR Cut: Find disconnected components in the undirected DFG
    let mut udfg = vec![KBitSet::<WORDS>::zero(); WORDS * 64];
    for i in 0..(WORDS * 64) {
        if footprint.contains(i) {
            udfg[i] = dfg[i].bitwise_and(footprint);
            for j in 0..(WORDS * 64) {
                if footprint.contains(j) && dfg[j].contains(i) {
                    let _ = udfg[i].set(j);
                }
            }
        }
    }

    // Transitive Closure for XOR Cut (Undirected)
    for k in 0..(WORDS * 64) {
        for i in 0..(WORDS * 64) {
            if udfg[i].contains(k) {
                let k_mask = udfg[k];
                udfg[i] = udfg[i].bitwise_or(k_mask);
            }
        }
    }

    let mut components = Vec::new();
    let mut remaining = footprint;
    while !remaining.is_empty() {
        let mut first = 0;
        for i in 0..(WORDS * 64) {
            if remaining.contains(i) {
                first = i;
                break;
            }
        }
        let mut comp = udfg[first];
        let _ = comp.set(first);
        let final_comp = comp.bitwise_and(remaining);
        components.push(final_comp);
        remaining = remaining.bitwise_and(final_comp.bitwise_not());
    }

    if components.len() > 1 {
        let mut children = Vec::new();
        for comp in components {
            children.push(mine_powl(dfg, comp, activity_names));
        }
        return PowlNode::Operator {
            operator: PowlOperator::XOR,
            children,
        };
    }

    // 2. SEQUENCE Cut: Find Strongly Connected Components (SCCs)
    let mut sccs = Vec::new();
    let mut tdfg = vec![KBitSet::<WORDS>::zero(); WORDS * 64];
    for i in 0..(WORDS * 64) {
        if footprint.contains(i) {
            tdfg[i] = dfg[i].bitwise_and(footprint);
        }
    }

    // BCINR Maximization: Use high-speed bitset-based SCC detection from utils
    let scc_masks = bcinr_core::scc::compute_sccs_generic::<WORDS>(&tdfg);

    // Filter scc_masks to only those in our current footprint and size > 1
    for mask in scc_masks {
        let intersection = mask.bitwise_and(footprint);
        if !intersection.is_empty() {
            // Count bits in intersection to ensure it's a real component
            let mut bits = 0;
            for i in 0..(WORDS * 64) {
                if intersection.contains(i) {
                    bits += 1;
                }
            }
            if bits > 0 {
                sccs.push(intersection);
            }
        }
    }

    if sccs.len() > 1 {
        // Generic SCC detection using reachability
        let mut closure = tdfg.clone();
        for k in 0..(WORDS * 64) {
            for i in 0..(WORDS * 64) {
                if closure[i].contains(k) {
                    let k_mask = closure[k];
                    closure[i] = closure[i].bitwise_or(k_mask);
                }
            }
        }

        // Topologically sort SCCs based on closure
        sccs.sort_by(|a, b| {
            let mut a_idx = 0;
            for i in 0..(WORDS * 64) {
                if a.contains(i) {
                    a_idx = i;
                    break;
                }
            }
            let mut b_idx = 0;
            for i in 0..(WORDS * 64) {
                if b.contains(i) {
                    b_idx = i;
                    break;
                }
            }

            if closure[a_idx].contains(b_idx) {
                std::cmp::Ordering::Less
            } else if closure[b_idx].contains(a_idx) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });

        let mut children = Vec::new();
        for scc in sccs {
            children.push(mine_powl(dfg, scc, activity_names));
        }
        return PowlNode::Operator {
            operator: PowlOperator::SEQUENCE,
            children,
        };
    }

    // 3. PARALLEL Cut (Complement graph)
    let mut cdfg = vec![KBitSet::<WORDS>::zero(); WORDS * 64];
    for i in 0..(WORDS * 64) {
        if footprint.contains(i) {
            for j in 0..(WORDS * 64) {
                if footprint.contains(j) && i != j && !dfg[i].contains(j) && !dfg[j].contains(i) {
                    let _ = cdfg[i].set(j);
                }
            }
        }
    }

    // Transitive Closure for Parallel Cut
    for k in 0..(WORDS * 64) {
        for i in 0..(WORDS * 64) {
            if cdfg[i].contains(k) {
                let k_mask = cdfg[k];
                cdfg[i] = cdfg[i].bitwise_or(k_mask);
            }
        }
    }

    let mut p_components = Vec::new();
    let mut remaining = footprint;
    while !remaining.is_empty() {
        let mut first = 0;
        for i in 0..(WORDS * 64) {
            if remaining.contains(i) {
                first = i;
                break;
            }
        }
        let mut comp = cdfg[first];
        let _ = comp.set(first);
        let final_comp = comp.bitwise_and(remaining);
        p_components.push(final_comp);
        remaining = remaining.bitwise_and(final_comp.bitwise_not());
    }

    if p_components.len() > 1 {
        let mut children = Vec::new();
        for comp in p_components {
            children.push(mine_powl(dfg, comp, activity_names));
        }
        return PowlNode::Operator {
            operator: PowlOperator::PARALLEL,
            children,
        };
    }

    // 4. Fallback: Choice Graph
    let mut nodes = Vec::new();
    let mut map = vec![0; WORDS * 64];
    for i in 0..(WORDS * 64) {
        if footprint.contains(i) {
            map[i] = nodes.len();
            let label = activity_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("Activity_{}", i));
            nodes.push(PowlNode::Transition {
                label: Some(label),
                id: i as u64,
            });
        }
    }

    let mut edges = Vec::new();
    for i in 0..(WORDS * 64) {
        if footprint.contains(i) {
            for j in 0..(WORDS * 64) {
                if footprint.contains(j) && dfg[i].contains(j) {
                    edges.push((map[i], map[j]));
                }
            }
        }
    }

    let num_nodes = nodes.len();
    PowlNode::ChoiceGraph {
        nodes,
        edges,
        start_nodes: vec![0],
        end_nodes: vec![if num_nodes == 0 { 0 } else { num_nodes - 1 }],
        empty_path: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nanosecond_inductive_miner_sequence() {
        let mut dfg = vec![KBitSet::<1>::zero(); 64];
        let _ = dfg[0].set(1); // 0 -> 1
        let _ = dfg[1].set(2); // 1 -> 2

        let mut footprint = KBitSet::<1>::zero();
        let _ = footprint.set(0);
        let _ = footprint.set(1);
        let _ = footprint.set(2);

        let names = vec!["A".to_string(), "B".to_string(), "C".to_string()];

        let ast = mine_powl(&dfg, footprint, &names);

        match ast {
            PowlNode::Operator { operator, children } => {
                assert_eq!(operator, PowlOperator::SEQUENCE);
                assert_eq!(children.len(), 3);
            }
            _ => panic!("Expected SEQUENCE operator"),
        }
    }

    #[test]
    fn test_nanosecond_inductive_miner_xor() {
        let mut dfg = vec![KBitSet::<1>::zero(); 64];
        // 0 -> 1
        // 2 -> 3
        let _ = dfg[0].set(1);
        let _ = dfg[2].set(3);

        let mut footprint = KBitSet::<1>::zero();
        let _ = footprint.set(0);
        let _ = footprint.set(1);
        let _ = footprint.set(2);
        let _ = footprint.set(3);

        let names = vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
            "D".to_string(),
        ];

        let ast = mine_powl(&dfg, footprint, &names);

        match ast {
            PowlNode::Operator { operator, children } => {
                assert_eq!(operator, PowlOperator::XOR);
                assert_eq!(children.len(), 2); // Two sequences
            }
            _ => panic!("Expected XOR operator"),
        }
    }

    #[test]
    fn test_discover_jtbd_13_model() {
        // jtbd_13: Start (0) -> Normal (1) -> End (3)
        let mut dfg = vec![KBitSet::<1>::zero(); 64];
        let _ = dfg[0].set(1); // Start -> Normal
        let _ = dfg[1].set(3); // Normal -> End

        let mut footprint = KBitSet::<1>::zero();
        let _ = footprint.set(0);
        let _ = footprint.set(1);
        let _ = footprint.set(3);

        let names = vec![
            "Start".to_string(),
            "Normal".to_string(),
            "Bypass".to_string(),
            "End".to_string(),
        ];

        let ast = mine_powl(&dfg, footprint, &names);

        // Should discover a SEQUENCE [Start, Normal, End]
        match ast {
            PowlNode::Operator { operator, children } => {
                assert_eq!(operator, PowlOperator::SEQUENCE);
                assert_eq!(children.len(), 3);
            }
            _ => panic!("Expected SEQUENCE operator for JTBD-13 model"),
        }
    }
}
