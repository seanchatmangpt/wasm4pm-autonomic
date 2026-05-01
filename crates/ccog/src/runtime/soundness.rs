//! Formal soundness checks for COG8 topologies (PRD v0.4).
//!
//! Provides reachability, proper completion, deadlock detection, and liveness
//! analysis for nonlinear cognitive graphs.

use crate::ids::NodeId;
use crate::runtime::cog8::{Cog8Edge, Cog8Row, Instinct};
use std::collections::VecDeque;

/// Soundness violation types.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoundnessError {
    /// Node is not reachable from the root (Node 0).
    UnreachableNode(NodeId),
    /// Path exists that does not end in a terminal (non-Ignore) response.
    NoTerminalResponse(NodeId),
    /// Cycle detected where no node has an exit to a terminal path.
    DeadlockCycle(Vec<NodeId>),
    /// Edge can never be executed because its guard_mask is unsatisfiable.
    UnexecutableEdge(usize),
}

/// Verify the formal soundness of a COG8 graph.
pub fn check_soundness(nodes: &[Cog8Row], edges: &[Cog8Edge]) -> Vec<SoundnessError> {
    let mut errors = Vec::new();
    let num_nodes = nodes.len();
    if num_nodes == 0 {
        return errors;
    }

    // 1. Reachability (BFS from Node 0)
    let mut adj = vec![Vec::new(); num_nodes];
    let mut rev_adj = vec![Vec::new(); num_nodes];
    for (i, edge) in edges.iter().enumerate() {
        let from = edge.from.0 as usize;
        let to = edge.to.0 as usize;
        if from < num_nodes && to < num_nodes {
            adj[from].push((to, i));
            rev_adj[to].push(from);
        }
    }

    let mut reachable = vec![false; num_nodes];
    let mut queue = VecDeque::new();
    reachable[0] = true;
    queue.push_back(0);

    while let Some(u) = queue.pop_front() {
        for &(v, _) in &adj[u] {
            if !reachable[v] {
                reachable[v] = true;
                queue.push_back(v);
            }
        }
    }

    for (i, r) in reachable.iter().enumerate().take(num_nodes) {
        if !*r {
            errors.push(SoundnessError::UnreachableNode(NodeId(i as u16)));
        }
    }

    // 2. Liveness: Can every edge be executed?
    // We simulate the monotonic growth of the completion mask.
    let mut possible_completed = 0u64;
    let mut changed = true;
    let mut fired_edges = vec![false; edges.len()];

    // We assume any bit NOT in the effect_masks can't be set by the graph itself,
    // but the initial completed_mask could be anything. For soundness check,
    // we assume we start with 0 and see if the graph can self-activate.
    while changed {
        changed = false;
        for (i, edge) in edges.iter().enumerate() {
            if !fired_edges[i] {
                // If the guard is satisfied by what we've completed so far
                if (possible_completed & edge.instr.guard_mask) == edge.instr.guard_mask {
                    let old = possible_completed;
                    possible_completed |= edge.instr.effect_mask;
                    fired_edges[i] = true;
                    if possible_completed != old {
                        changed = true;
                    }
                }
            }
        }
    }

    for (i, fired) in fired_edges.iter().enumerate() {
        if !fired {
            errors.push(SoundnessError::UnexecutableEdge(i));
        }
    }

    // 3. Proper Completion & Deadlock
    // Every reachable node must have a path to a node with response != Instinct::Ignore.
    let mut leads_to_terminal = vec![false; num_nodes];
    for i in 0..num_nodes {
        if nodes[i].response != Instinct::Ignore {
            leads_to_terminal[i] = true;
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..num_nodes {
            if !leads_to_terminal[i] {
                for &(v, _) in &adj[i] {
                    if leads_to_terminal[v] {
                        leads_to_terminal[i] = true;
                        changed = true;
                        break;
                    }
                }
            }
        }
    }

    for i in 0..num_nodes {
        if reachable[i] && !leads_to_terminal[i] {
            // Check if it's part of a cycle or just a dead end
            // (Both are violations of proper completion in this model)
            errors.push(SoundnessError::NoTerminalResponse(NodeId(i as u16)));
        }
    }

    // 4. Deadlock Check (Cycles without an exit)
    // Find Strongly Connected Components (SCCs).
    // If an SCC has no edges leading to nodes outside the SCC that lead to a terminal, it's a deadlock.
    let sccs = find_sccs(num_nodes, &adj);
    for scc in sccs {
        if scc.len() > 1 || (scc.len() == 1 && has_self_loop(scc[0], &adj)) {
            // It's a cycle. Does it have an exit to a terminal path?
            let mut has_exit_to_terminal = false;
            for &u in &scc {
                for &(v, _) in &adj[u] {
                    if !scc.contains(&v) && leads_to_terminal[v] {
                        has_exit_to_terminal = true;
                        break;
                    }
                }
                if has_exit_to_terminal {
                    break;
                }
                // Also, if the node itself is terminal, it's fine
                if nodes[u].response != Instinct::Ignore {
                    has_exit_to_terminal = true;
                    break;
                }
            }

            if !has_exit_to_terminal {
                errors.push(SoundnessError::DeadlockCycle(
                    scc.into_iter().map(|idx| NodeId(idx as u16)).collect(),
                ));
            }
        }
    }

    errors
}

fn find_sccs(num_nodes: usize, adj: &[Vec<(usize, usize)>]) -> Vec<Vec<usize>> {
    let mut index = 0;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; num_nodes];
    let mut indices = vec![-1; num_nodes];
    let mut lowlink = vec![-1; num_nodes];
    let mut sccs = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn strongconnect(
        u: usize,
        index: &mut i32,
        stack: &mut Vec<usize>,
        on_stack: &mut [bool],
        indices: &mut [i32],
        lowlink: &mut [i32],
        adj: &[Vec<(usize, usize)>],
        sccs: &mut Vec<Vec<usize>>,
    ) {
        indices[u] = *index;
        lowlink[u] = *index;
        *index += 1;
        stack.push(u);
        on_stack[u] = true;

        for &(v, _) in &adj[u] {
            if indices[v] == -1 {
                strongconnect(v, index, stack, on_stack, indices, lowlink, adj, sccs);
                lowlink[u] = lowlink[u].min(lowlink[v]);
            } else if on_stack[v] {
                lowlink[u] = lowlink[u].min(indices[v]);
            }
        }

        if lowlink[u] == indices[u] {
            let mut scc = Vec::new();
            while let Some(v) = stack.pop() {
                on_stack[v] = false;
                scc.push(v);
                if u == v {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    for i in 0..num_nodes {
        if indices[i] == -1 {
            strongconnect(
                i,
                &mut index,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlink,
                adj,
                &mut sccs,
            );
        }
    }

    sccs
}

fn has_self_loop(u: usize, adj: &[Vec<(usize, usize)>]) -> bool {
    adj[u].iter().any(|&(v, _)| v == u)
}
