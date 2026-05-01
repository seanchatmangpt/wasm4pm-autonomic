//! Wil van der Aalst Alignment-based Conformance Checker (Phase 12).
#![allow(clippy::disallowed_types)]
//!
//! Provides structural alignment between live `EvidenceLedger` traces (recorded as `Powl64`
//! route proofs) and the admitted `CompiledCcogConfig` topology.
//!
//! - **Fitness**: Ratio of observed trace steps that are admissible in the topology.
//! - **Precision**: Ratio of topology edges that are exercised by the ledger.
//! - **Generalization**: Ability of the model to handle unseen but lawful behavior.
//! - **Simplicity**: Occam's razor for the COG8 topology.
//! - **False Closures**: Identification of traces that terminate on non-sink nodes.

use crate::ids::{EdgeId, NodeId};
use crate::powl64::Powl64;
use crate::runtime::cog8::EdgeKind;
use crate::runtime::CompiledCcogConfig;
use std::collections::{HashMap, HashSet};

/// Collection of recorded cognitive traces for conformance analysis.
#[derive(Debug, Clone, Default)]
pub struct EvidenceLedger {
    /// Sequence of route proofs observed in the live environment.
    pub traces: Vec<Powl64>,
}

impl EvidenceLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a trace to the ledger.
    pub fn record(&mut self, trace: Powl64) {
        self.traces.push(trace);
    }
}

/// Conformance metrics for a given configuration and ledger.
#[derive(Debug, Clone, PartialEq)]
pub struct ConformanceReport {
    /// Does the trace follow the graph? [0.0, 1.0]
    pub fitness: f64,
    /// Does the graph allow only the trace? [0.0, 1.0]
    pub precision: f64,
    /// Does the model generalize beyond the specific traces? [0.0, 1.0]
    pub generalization: f64,
    /// Is the model structuraly minimal? [0.0, 1.0]
    pub simplicity: f64,
    /// Nodes where traces terminated unexpectedly (early).
    pub false_closures: Vec<NodeId>,
}

/// Calculate conformance metrics by aligning traces to topology.
pub fn check_conformance<const N: usize, const E: usize>(
    config: &CompiledCcogConfig<N, E>,
    ledger: &EvidenceLedger,
) -> ConformanceReport {
    if ledger.traces.is_empty() {
        return ConformanceReport {
            fitness: 1.0, // Vacuously fit
            precision: 0.0,
            generalization: 0.0,
            simplicity: calculate_simplicity(config),
            false_closures: Vec::new(),
        };
    }

    let mut total_cells = 0;
    let mut fit_cells = 0;

    // Track visits and usage
    let mut node_visit_counts = HashMap::new();
    let mut exercised_edges_per_node = HashMap::<NodeId, HashSet<EdgeId>>::new();
    let mut false_closures = Vec::new();

    // Map NodeId to its outgoing edges in the config
    let mut node_to_config_edges = HashMap::<NodeId, HashSet<EdgeId>>::new();
    let mut sink_nodes = HashSet::new();
    for i in 0..N {
        sink_nodes.insert(NodeId(i as u16));
    }

    for edge in config.edges.iter() {
        if edge.kind != EdgeKind::None {
            node_to_config_edges
                .entry(edge.from)
                .or_default()
                .insert(edge.instr.edge_id);
            if edge.kind != EdgeKind::Loop && edge.kind != EdgeKind::Blocking {
                sink_nodes.remove(&edge.from);
            }
        }
    }

    for trace in &ledger.traces {
        total_cells += trace.cells.len();

        for cell in &trace.cells {
            // Check if the cell aligns with an edge in the config.
            let mut found_match = false;
            for edge in config.edges.iter() {
                if edge.kind != EdgeKind::None
                    && edge.instr.edge_id == cell.edge_id
                    && edge.from == cell.from_node
                    && edge.to == cell.to_node
                {
                    found_match = true;
                    exercised_edges_per_node
                        .entry(cell.from_node)
                        .or_default()
                        .insert(cell.edge_id);
                    break;
                }
            }
            if found_match {
                fit_cells += 1;
            }

            // Track visits for generalization
            *node_visit_counts.entry(cell.to_node).or_insert(0) += 1;
        }

        // Count the starting node of each trace
        if let Some(first_cell) = trace.cells.first() {
            *node_visit_counts.entry(first_cell.from_node).or_insert(0) += 1;
        }

        // False Closure detection: check the last node of the trace.
        if let Some(last_cell) = trace.cells.last() {
            let last_node = last_cell.to_node;
            if !sink_nodes.contains(&last_node) {
                false_closures.push(last_node);
            }
        }
    }

    // 1. Fitness (Replay Fitness)
    let fitness = if total_cells > 0 {
        fit_cells as f64 / total_cells as f64
    } else {
        1.0
    };

    // 2. Precision
    // Ratio of exercised edges to total possible edges from all visited nodes.
    let mut total_possible_edges = 0;
    let mut total_exercised_edges = 0;
    for node in node_visit_counts.keys() {
        if let Some(possible) = node_to_config_edges.get(node) {
            total_possible_edges += possible.len();
            if let Some(exercised) = exercised_edges_per_node.get(node) {
                total_exercised_edges += exercised.len();
            }
        }
    }
    let precision = if total_possible_edges > 0 {
        total_exercised_edges as f64 / total_possible_edges as f64
    } else {
        1.0
    };

    // 3. Generalization
    // Van der Aalst metric: 1 - (nodes visited once / total nodes visited)
    let total_distinct_nodes = node_visit_counts.len();
    let nodes_visited_once = node_visit_counts.values().filter(|&&v| v == 1).count();
    let generalization = if total_distinct_nodes > 0 {
        1.0 - (nodes_visited_once as f64 / total_distinct_nodes as f64)
    } else {
        1.0
    };

    // 4. Simplicity
    let simplicity = calculate_simplicity(config);

    ConformanceReport {
        fitness,
        precision,
        generalization,
        simplicity,
        false_closures,
    }
}

/// Internal utility to calculate structural simplicity of the graph.
fn calculate_simplicity<const N: usize, const E: usize>(config: &CompiledCcogConfig<N, E>) -> f64 {
    let mut active_nodes = HashSet::new();
    let mut active_edges_count = 0;
    for edge in config.edges.iter() {
        if edge.kind != EdgeKind::None {
            active_nodes.insert(edge.from);
            active_nodes.insert(edge.to);
            active_edges_count += 1;
        }
    }

    let active_nodes_count = active_nodes.len();
    if active_nodes_count == 0 {
        return 1.0;
    }

    // Occam's razor: favor models with fewer "extra" edges relative to nodes.
    // Simplicity = Nodes / (Nodes + Max(0, Edges - Nodes + 1))
    let extra_edges = if active_edges_count >= active_nodes_count {
        active_edges_count - active_nodes_count + 1
    } else {
        0
    };

    active_nodes_count as f64 / (active_nodes_count + extra_edges) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::*;
    use crate::powl64::Powl64RouteCell;
    use crate::runtime::cog8::{
        Cog8Edge, Cog8Row, CollapseFn, EdgeKind, Instinct, Powl8Instr, Powl8Op,
    };

    fn create_conformance_config() -> CompiledCcogConfig<2, 1> {
        let node0 = Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(1),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::None,
            var_ids: [FieldId::NONE; 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 0,
            response: Instinct::Ignore,
            priority: 0,
        };
        let node1 = Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(2),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::None,
            var_ids: [FieldId::NONE; 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 1,
            response: Instinct::Settle,
            priority: 0,
        };

        let edge = Cog8Edge {
            from: NodeId(0),
            to: NodeId(1),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Choice,
                collapse_fn: CollapseFn::None,
                node_id: NodeId(1),
                edge_id: EdgeId(1),
                guard_mask: 0,
                effect_mask: 1,
            },
        };

        CompiledCcogConfig {
            pack: crate::runtime::cog8::LoadedFieldPack {
                id: PackId(1),
                name: "test",
                ontology_profile: vec![],
                digest_urn: "urn:test".to_string(),
            },
            nodes: [node0, node1],
            edges: [edge],
            mcp_projections: crate::runtime::mcp::MCPProjectionTable::new(),
        }
    }

    #[test]
    fn test_perfect_conformance() {
        let config = create_conformance_config();
        let mut ledger = EvidenceLedger::new();
        let mut trace = Powl64::new();
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });
        ledger.record(trace);

        let report = check_conformance(&config, &ledger);
        assert_eq!(report.fitness, 1.0);
        assert_eq!(report.precision, 1.0);
        assert_eq!(report.simplicity, 1.0);
        assert!(report.false_closures.is_empty());
    }

    #[test]
    fn test_fitness_violation() {
        let config = create_conformance_config();
        let mut ledger = EvidenceLedger::new();
        let mut trace = Powl64::new();
        trace.extend(Powl64RouteCell {
            edge_id: EdgeId(99), // Non-existent edge
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });
        ledger.record(trace);

        let report = check_conformance(&config, &ledger);
        assert_eq!(report.fitness, 0.0);
    }

    #[test]
    fn test_generalization_and_simplicity() {
        let config = create_conformance_config();
        let mut ledger = EvidenceLedger::new();

        let mut trace1 = Powl64::new();
        trace1.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });
        ledger.record(trace1);

        let report1 = check_conformance(&config, &ledger);
        assert_eq!(report1.generalization, 0.0);

        let mut trace2 = Powl64::new();
        trace2.extend(Powl64RouteCell {
            edge_id: EdgeId(1),
            from_node: NodeId(0),
            to_node: NodeId(1),
            ..Default::default()
        });
        ledger.record(trace2);

        let report2 = check_conformance(&config, &ledger);
        assert_eq!(report2.generalization, 1.0);
    }
}
