//! Cost-based alignment between observed traces and COG8 topologies.
#![allow(clippy::disallowed_types)]
//!
//! Implements optimal alignment searching with skip/insert costs.
//! Cost(skip) = 10 (Move in model only)
//! Cost(insert) = 15 (Move in log only)

use crate::ids::NodeId;
use crate::powl64::Powl64;
use crate::runtime::cog8::EdgeKind;
use crate::runtime::CompiledCcogConfig;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Cost of skipping a model transition (move in model only).
pub const COST_SKIP: u32 = 10;
/// Cost of inserting an observed transition (move in log only).
pub const COST_INSERT: u32 = 15;

/// Result of the alignment process.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignmentResult {
    /// Total minimal cost to align the trace to the model.
    pub cost: u32,
    /// Fitness score [0.0, 1.0] where 1.0 is perfect alignment.
    pub fitness: f64,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct SearchState {
    cost: u32,
    trace_idx: usize,
    model_node: NodeId,
}

impl Ord for SearchState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap for Dijkstra: smaller cost has higher priority.
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.trace_idx.cmp(&other.trace_idx))
    }
}

impl PartialOrd for SearchState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Engine for calculating optimal alignments between logs and models.
pub struct AlignmentEngine;

impl AlignmentEngine {
    /// Calculate the optimal alignment between a recorded `Powl64` trace
    /// and a `CompiledCcogConfig` topology.
    ///
    /// Instead of simple trace equality, this calculates the 'optimal alignment'
    /// by finding a path of moves that minimizes deviations.
    pub fn align<const N: usize, const E: usize>(
        trace: &Powl64,
        topology: &CompiledCcogConfig<N, E>,
    ) -> AlignmentResult {
        if trace.cells.is_empty() {
            return AlignmentResult {
                cost: 0,
                fitness: 1.0,
            };
        }

        let mut pq = BinaryHeap::new();
        let mut visited = HashMap::new();

        // Initial state: Start from the beginning of the trace and the model root.
        // Process Mining Standard: Alignment usually starts from the model's entry node.
        pq.push(SearchState {
            cost: 0,
            trace_idx: 0,
            model_node: NodeId(0), // Anchor to model root
        });

        let mut min_total_cost = u32::MAX;

        while let Some(SearchState {
            cost,
            trace_idx,
            model_node,
        }) = pq.pop()
        {
            if cost >= min_total_cost {
                continue;
            }

            if let Some(&best_cost) = visited.get(&(trace_idx, model_node)) {
                if cost >= best_cost {
                    continue;
                }
            }
            visited.insert((trace_idx, model_node), cost);

            // Goal reached: all trace cells processed.
            if trace_idx == trace.cells.len() {
                if cost < min_total_cost {
                    min_total_cost = cost;
                }
                // We could continue to see if skipping some more model edges helps,
                // but usually finishing the trace is the goal.
                continue;
            }

            let current_cell = &trace.cells[trace_idx];

            // 1. Synchronous Move: trace cell matches a model edge.
            // Cost: 0.
            for edge in topology.edges.iter() {
                if edge.kind != EdgeKind::None
                    && edge.kind != EdgeKind::Blocking
                    && edge.from == model_node
                {
                    // Match based on edge_id and target node.
                    if edge.instr.edge_id == current_cell.edge_id && edge.to == current_cell.to_node
                    {
                        pq.push(SearchState {
                            cost,
                            trace_idx: trace_idx + 1,
                            model_node: edge.to,
                        });
                    }
                }
            }

            // 2. Move in Model (Skip): we take a model edge but stay at the same trace position.
            // Cost: 10.
            for edge in topology.edges.iter() {
                if edge.kind != EdgeKind::None
                    && edge.kind != EdgeKind::Blocking
                    && edge.from == model_node
                {
                    pq.push(SearchState {
                        cost: cost + COST_SKIP,
                        trace_idx,
                        model_node: edge.to,
                    });
                }
            }

            // 3. Move in Log (Insert): we take the trace cell but stay at the same model node.
            // Cost: 15.
            pq.push(SearchState {
                cost: cost + COST_INSERT,
                trace_idx: trace_idx + 1,
                model_node,
            });
        }

        // Calculate fitness.
        // We use a denominator based on the cost of full insertion.
        let log_cost = trace.cells.len() as u32 * COST_INSERT;
        let fitness = if log_cost > 0 {
            (1.0 - (min_total_cost as f64 / log_cost as f64)).max(0.0)
        } else {
            1.0
        };

        AlignmentResult {
            cost: min_total_cost,
            fitness,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::*;
    use crate::powl64::Powl64RouteCell;
    use crate::runtime::cog8::{Cog8Edge, Cog8Row, LoadedFieldPack, Powl8Instr, Powl8Op};

    fn mock_config() -> CompiledCcogConfig<3, 2> {
        let node0 = Cog8Row {
            priority: 0,
            ..Cog8Row::default()
        };
        let node1 = Cog8Row {
            priority: 0,
            ..Cog8Row::default()
        };
        let node2 = Cog8Row {
            priority: 0,
            ..Cog8Row::default()
        };

        // Edge 0: 0 -> 1
        let edge0 = Cog8Edge {
            from: NodeId(0),
            to: NodeId(1),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Choice,
                edge_id: EdgeId(1),
                ..Powl8Instr::default()
            },
        };

        // Edge 1: 1 -> 2
        let edge1 = Cog8Edge {
            from: NodeId(1),
            to: NodeId(2),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Choice,
                edge_id: EdgeId(2),
                ..Powl8Instr::default()
            },
        };

        CompiledCcogConfig {
            pack: LoadedFieldPack {
                id: PackId(1),
                name: "test",
                ontology_profile: vec![],
                digest_urn: "urn:test".to_string(),
            },
            nodes: [node0, node1, node2],
            edges: [edge0, edge1],
            mcp_projections: crate::runtime::mcp::MCPProjectionTable::new(),
        }
    }

    #[test]
    fn test_alignment_perfect() {
        let config = mock_config();
        let mut trace = Powl64::new();
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(2),
            from_node: NodeId(1),
            to_node: NodeId(2),
            ..Default::default()
        });

        let result = AlignmentEngine::align(&trace, &config);
        assert_eq!(result.cost, 0);
        assert_eq!(result.fitness, 1.0);
    }

    #[test]
    fn test_alignment_skip() {
        let config = mock_config();
        let mut trace = Powl64::new();
        // Skip Edge 1 (0->1), only have Edge 2 (1->2).
        // To align this, we must move in model from 0 to 1 (cost 10),
        // then move in both for Edge 2.
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(2),
            from_node: NodeId(1),
            to_node: NodeId(2),
            ..Default::default()
        });

        let result = AlignmentEngine::align(&trace, &config);
        assert_eq!(result.cost, 10);
        // fitness = 1 - (10 / 15) = 1 - 0.666 = 0.333
        assert!(result.fitness < 0.34 && result.fitness > 0.33);
    }

    #[test]
    fn test_alignment_insert() {
        let config = mock_config();
        let mut trace = Powl64::new();
        // Normal path: 0->1->2
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });
        // Extra transition: 1->99 (Insert)
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(99),
            from_node: NodeId(1),
            to_node: NodeId(99),
            ..Default::default()
        });
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(2),
            from_node: NodeId(1),
            to_node: NodeId(2),
            ..Default::default()
        });

        let result = AlignmentEngine::align(&trace, &config);
        assert_eq!(result.cost, 15);
    }
}
