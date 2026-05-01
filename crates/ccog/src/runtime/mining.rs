//! Heuristic Process Discovery Engine (Wil van der Aalst Agent).
//!
//! This module implements a zero-allocation discovery engine that mines
//! `EvidenceLedger` (POWL64) traces to reconstruct heuristic process models.
//!
//! It maps collaborative routes to XES-standard event logs and identifies
//! 'Frequent Routes' for potential COG8 promotion.

use crate::powl64::{PartnerId, Powl64, ProjectionTarget};
use crate::runtime::cog8::{CollapseFn, NodeId};

/// Maximum number of unique nodes supported by the zero-allocation mining workspace.
pub const MAX_MINING_NODES: usize = 64;

/// A collection of POWL64 traces representing an Evidence Ledger.
pub struct EvidenceLedger<'a> {
    /// Sequence of process traces.
    pub traces: &'a [Powl64],
}

/// Heuristic dependency measure thresholds.
#[derive(Debug, Clone, Copy)]
pub struct DiscoveryConfig {
    /// Dependency threshold (0.0 to 1.0).
    pub dependency_threshold: f32,
    /// Minimum frequency for a node/edge to be considered.
    pub frequency_threshold: u32,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            dependency_threshold: 0.1,
            frequency_threshold: 0,
        }
    }
}

/// Zero-allocation workspace for frequency counting and dependency calculation.
pub struct MiningWorkspace {
    /// Frequency of each node.
    pub frequencies: [u32; MAX_MINING_NODES],
    /// Frequency of directly-follows relations (from, to).
    pub follows: [[u32; MAX_MINING_NODES]; MAX_MINING_NODES],
    /// Mapping from node index to NodeId.
    pub node_map: [NodeId; MAX_MINING_NODES],
    /// Number of unique nodes discovered.
    pub node_count: usize,
}

impl Default for MiningWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl MiningWorkspace {
    /// Create a new, empty mining workspace.
    pub const fn new() -> Self {
        Self {
            frequencies: [0; MAX_MINING_NODES],
            follows: [[0; MAX_MINING_NODES]; MAX_MINING_NODES],
            node_map: [NodeId(0xFFFF); MAX_MINING_NODES],
            node_count: 0,
        }
    }

    /// Reset the workspace for a new discovery run.
    pub fn reset(&mut self) {
        self.frequencies = [0; MAX_MINING_NODES];
        self.follows = [[0; MAX_MINING_NODES]; MAX_MINING_NODES];
        self.node_map = [NodeId(0xFFFF); MAX_MINING_NODES];
        self.node_count = 0;
    }

    fn get_or_insert_index(&mut self, node: NodeId) -> Option<usize> {
        for i in 0..self.node_count {
            if self.node_map[i] == node {
                return Some(i);
            }
        }
        if self.node_count < MAX_MINING_NODES {
            let i = self.node_count;
            self.node_map[i] = node;
            self.node_count += 1;
            Some(i)
        } else {
            None
        }
    }
}

/// XES-standard event log mapping.
#[derive(Debug, Clone, Copy)]
pub struct XesEvent {
    /// Canonical name (NodeId).
    pub concept_name: NodeId,
    /// Collaborative partner.
    pub partner: PartnerId,
    /// Projection target.
    pub target: ProjectionTarget,
    /// Cognitive collapse attribution.
    pub collapse_fn: CollapseFn,
    /// Cryptographic chain head (pseudo-timestamp).
    pub chain_head: u64,
}

/// Heuristic Miner discovery engine.
pub struct HeuristicMiner;

impl HeuristicMiner {
    /// Mines the EvidenceLedger to populate the MiningWorkspace.
    pub fn mine(ledger: &EvidenceLedger, workspace: &mut MiningWorkspace) {
        for trace in ledger.traces {
            let mut last_idx: Option<usize> = None;

            for cell in &trace.cells {
                // Record node frequency (at 'to_node')
                if let Some(idx) = workspace.get_or_insert_index(cell.to_node) {
                    workspace.frequencies[idx] += 1;

                    // Record follows frequency
                    if let Some(prev) = last_idx {
                        workspace.follows[prev][idx] += 1;
                    }
                    last_idx = Some(idx);
                }
            }
        }
    }

    /// Calculate the dependency measure between two nodes.
    ///
    /// dep(A, B) = (|A > B| - |B > A|) / (|A > B| + |B > A| + 1)
    pub fn dependency(workspace: &MiningWorkspace, from_idx: usize, to_idx: usize) -> f32 {
        let a_follows_b = workspace.follows[from_idx][to_idx] as f32;
        let b_follows_a = workspace.follows[to_idx][from_idx] as f32;

        if from_idx == to_idx {
            // Self-loop dependency
            a_follows_b / (a_follows_b + 1.0)
        } else {
            (a_follows_b - b_follows_a) / (a_follows_b + b_follows_a + 1.0)
        }
    }

    /// Identify frequent routes for potential COG8 promotion.
    ///
    /// Yields pairs of (NodeId, NodeId) that exceed the dependency threshold.
    pub fn discover_frequent_routes<'a>(
        workspace: &'a MiningWorkspace,
        config: &'a DiscoveryConfig,
    ) -> impl Iterator<Item = (NodeId, NodeId)> + 'a {
        (0..workspace.node_count).flat_map(move |i| {
            (0..workspace.node_count).filter_map(move |j| {
                let dep = Self::dependency(workspace, i, j);
                if dep >= config.dependency_threshold
                    && workspace.follows[i][j] >= config.frequency_threshold
                {
                    Some((workspace.node_map[i], workspace.node_map[j]))
                } else {
                    None
                }
            })
        })
    }

    /// Map a POWL64 trace to XES-standard events.
    pub fn map_to_xes(trace: &Powl64) -> impl Iterator<Item = XesEvent> + '_ {
        trace.cells.iter().map(|cell| XesEvent {
            concept_name: cell.to_node,
            partner: cell.partner_id,
            target: cell.projection_target,
            collapse_fn: cell.collapse_fn,
            chain_head: cell.chain_head,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EdgeId;
    use crate::powl64::{Polarity, Powl64RouteCell};
    use crate::runtime::cog8::{EdgeKind, NodeId};

    #[test]
    fn test_heuristic_mining_logic() {
        let mut trace = Powl64::new();

        // Node 1 -> Node 2 -> Node 3
        trace.extend(Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(0),
            to_node: NodeId(1),
            edge_id: EdgeId(1),
            edge_kind: EdgeKind::Choice,
            collapse_fn: CollapseFn::ExpertRule,
            polarity: Polarity::Positive,
            projection_target: ProjectionTarget::NoOp,
            partner_id: PartnerId::NONE,
            input_digest: 0,
            args_digest: 0,
            result_digest: 0,
            prior_chain: 0,
            chain_head: 100,
        });
        trace.extend(Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(1),
            to_node: NodeId(2),
            edge_id: EdgeId(2),
            edge_kind: EdgeKind::Choice,
            collapse_fn: CollapseFn::ExpertRule,
            polarity: Polarity::Positive,
            projection_target: ProjectionTarget::NoOp,
            partner_id: PartnerId::NONE,
            input_digest: 0,
            args_digest: 0,
            result_digest: 0,
            prior_chain: 100,
            chain_head: 200,
        });
        trace.extend(Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(2),
            to_node: NodeId(3),
            edge_id: EdgeId(3),
            edge_kind: EdgeKind::Choice,
            collapse_fn: CollapseFn::ExpertRule,
            polarity: Polarity::Positive,
            projection_target: ProjectionTarget::NoOp,
            partner_id: PartnerId::NONE,
            input_digest: 0,
            args_digest: 0,
            result_digest: 0,
            prior_chain: 200,
            chain_head: 300,
        });

        let ledger = EvidenceLedger { traces: &[trace] };

        let mut workspace = MiningWorkspace::new();
        HeuristicMiner::mine(&ledger, &mut workspace);

        assert_eq!(workspace.node_count, 3);
        assert_eq!(workspace.frequencies[0], 1); // Node 1
        assert_eq!(workspace.frequencies[1], 1); // Node 2
        assert_eq!(workspace.frequencies[2], 1); // Node 3

        // Dependency between Node 1 and Node 2
        // A=1, B=2. |A>B|=1, |B>A|=0. dep = (1-0)/(1+0+1) = 0.5
        let dep_1_2 = HeuristicMiner::dependency(&workspace, 0, 1);
        assert_eq!(dep_1_2, 0.5);

        let config = DiscoveryConfig {
            dependency_threshold: 0.4,
            frequency_threshold: 1,
        };

        let routes: Vec<_> =
            HeuristicMiner::discover_frequent_routes(&workspace, &config).collect();
        assert_eq!(routes.len(), 2);
        assert!(routes.contains(&(NodeId(1), NodeId(2))));
        assert!(routes.contains(&(NodeId(2), NodeId(3))));
    }

    #[test]
    fn test_map_to_xes() {
        let mut trace = Powl64::new();
        trace.extend(Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(0),
            to_node: NodeId(10),
            edge_id: EdgeId(1),
            edge_kind: EdgeKind::Choice,
            collapse_fn: CollapseFn::ExpertRule,
            polarity: Polarity::Positive,
            projection_target: ProjectionTarget::NoOp,
            partner_id: PartnerId::NONE,
            input_digest: 0,
            args_digest: 0,
            result_digest: 0,
            prior_chain: 0,
            chain_head: 555,
        });

        let events: Vec<_> = HeuristicMiner::map_to_xes(&trace).collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].concept_name, NodeId(10));
        assert_eq!(events[0].chain_head, 555);
    }
}
