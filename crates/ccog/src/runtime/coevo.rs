//! Coevolutionary Enhancement (Wil van der Aalst Agent - Phase 12).
#![allow(clippy::disallowed_types)]
//!
//! Provides automated model enhancement by suggesting 'shortcuts' through
//! high-latency cognitive delegations (MCP/A2A) based on frequent trace patterns.

use crate::ids::*;
use crate::powl64::ProjectionTarget;
use crate::runtime::hitl::ExternalBurden;
use crate::runtime::mining::{DiscoveryConfig, EvidenceLedger, HeuristicMiner, MiningWorkspace};
use crate::runtime::{
    cog8::{Cog8Edge, Cog8Row, CollapseFn, EdgeId, EdgeKind, NodeId, Powl8Instr, Powl8Op},
    CompiledCcogConfig,
};

/// A suggested improvement to the cognitive graph.
#[derive(Debug, Clone)]
pub struct CandidateChunk {
    /// Descriptive name for the enhancement.
    pub name: String,
    /// Proposed new node logic.
    pub suggested_node: Cog8Row,
    /// Proposed new edge logic.
    pub suggested_edge: Cog8Edge,
    /// Node being bypassed.
    pub bypass_node: NodeId,
    /// Estimated reduction in external burden cost.
    pub burden_reduction: u64,
    /// Reason for the suggestion.
    pub reason: String,
}

/// Enhancement engine based on Wil van der Aalst's principles.
pub struct CoevolutionEnhancer;

impl CoevolutionEnhancer {
    /// Generate candidate chunks to reduce bottlenecking by bypassing high-latency delegations.
    pub fn generate_candidate_chunks<const N: usize, const E: usize>(
        config: &CompiledCcogConfig<N, E>,
        ledger: &EvidenceLedger,
        workspace: &mut MiningWorkspace,
    ) -> Vec<CandidateChunk> {
        let mut candidates = Vec::new();

        // 1. Mine frequent routes from the ledger.
        let discovery_config = DiscoveryConfig::default();
        workspace.reset();
        HeuristicMiner::mine(ledger, workspace);
        let frequent_routes: Vec<_> =
            HeuristicMiner::discover_frequent_routes(workspace, &discovery_config).collect();

        // 2. Identify high-burden external delegations (MCP/A2A/HITL).
        // We use the ledger traces to identify which nodes result in external projections.
        let mut latent_nodes = std::collections::HashMap::new();
        for trace in ledger.traces {
            for cell in &trace.cells {
                if cell.projection_target != ProjectionTarget::NoOp {
                    latent_nodes.insert(cell.to_node, cell.projection_target);
                }
            }
        }

        // 3. Look for A -> B (external) -> C patterns in frequent routes.
        for &(a, b) in &frequent_routes {
            if let Some(&target) = latent_nodes.get(&b) {
                for &(b2, c) in &frequent_routes {
                    if b == b2 && a != c {
                        // Pattern A -> B (external burden) -> C discovered.
                        // Suggest a shortcut A -> C.

                        // Look up the row for node C to copy its response and logic.
                        if let Some(c_row) = config.nodes.get(c.0 as usize) {
                            let mut suggested_node = *c_row;
                            // Update predecessor to A, bypassing B.
                            suggested_node.predecessor_mask = 1 << (a.0 % 64);
                            suggested_node.rule_id = RuleId(b.0 + 1000); // New synthetic rule ID
                            suggested_node.collapse_fn = CollapseFn::Chunking;

                            let suggested_edge = Cog8Edge {
                                from: a,
                                to: c,
                                kind: EdgeKind::Choice,
                                instr: Powl8Instr {
                                    op: Powl8Op::Choice,
                                    collapse_fn: CollapseFn::Chunking,
                                    node_id: c,
                                    edge_id: EdgeId(b.0 + 1000),
                                    guard_mask: 1 << (a.0 % 64),
                                    effect_mask: 1 << (c.0 % 64),
                                },
                            };

                            let burden_reduction = ExternalBurden::cost(target);

                            candidates.push(CandidateChunk {
                                name: format!("Bypass-External-{}", b.0),
                                suggested_node,
                                suggested_edge,
                                bypass_node: b,
                                burden_reduction,
                                reason: format!(
                                    "Frequent route A({}) -> B({}) -> C({}) detected. B is high-latency external delegation ({:?}) with cost {}. Suggesting local shortcut A -> C.",
                                    a.0, b.0, c.0, target, burden_reduction
                                ),
                            });
                        }
                    }
                }
            }
        }

        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::powl64::{PartnerId, Polarity, Powl64RouteCell};
    use crate::runtime::cog8::{Instinct, LoadedFieldPack};
    use crate::runtime::MCPProjectionTable;

    #[test]
    fn test_enhancement_logic() {
        // Setup a mock config with Node 0 -> Node 1 (MCP) -> Node 2
        let pack = LoadedFieldPack {
            id: PackId(1),
            name: "test",
            ontology_profile: vec![],
            digest_urn: "urn:test".to_string(),
        };

        let node0 = Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(0),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::ExpertRule,
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
            rule_id: RuleId(1),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [FieldId::NONE; 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 1,
            response: Instinct::Ask,
            priority: 0,
        };
        let node2 = Cog8Row {
            pack_id: PackId(1),
            group_id: GroupId(1),
            rule_id: RuleId(2),
            breed_id: BreedId(1),
            collapse_fn: CollapseFn::ExpertRule,
            var_ids: [FieldId::NONE; 8],
            required_mask: 0,
            forbidden_mask: 0,
            predecessor_mask: 2,
            response: Instinct::Settle,
            priority: 0,
        };

        let edge01 = Cog8Edge {
            from: NodeId(0),
            to: NodeId(1),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Choice,
                collapse_fn: CollapseFn::ExpertRule,
                node_id: NodeId(1),
                edge_id: EdgeId(1),
                guard_mask: 1,
                effect_mask: 2,
            },
        };
        let edge12 = Cog8Edge {
            from: NodeId(1),
            to: NodeId(2),
            kind: EdgeKind::Choice,
            instr: Powl8Instr {
                op: Powl8Op::Choice,
                collapse_fn: CollapseFn::ExpertRule,
                node_id: NodeId(2),
                edge_id: EdgeId(2),
                guard_mask: 2,
                effect_mask: 4,
            },
        };

        let config = CompiledCcogConfig::<3, 2> {
            pack,
            nodes: [node0, node1, node2],
            edges: [edge01, edge12],
            mcp_projections: MCPProjectionTable::new(),
        };

        // Setup a ledger with trace: 0 -> 1 (MCP) -> 2
        let mut trace = crate::powl64::Powl64::new();
        trace.extend(Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(0),
            to_node: NodeId(0), // Initial
            ..Default::default()
        });
        trace.extend(Powl64RouteCell {
            graph_id: 1,
            from_node: NodeId(0),
            to_node: NodeId(1),
            edge_id: EdgeId(1),
            edge_kind: EdgeKind::Choice,
            collapse_fn: CollapseFn::ExpertRule,
            polarity: Polarity::Positive,
            projection_target: ProjectionTarget::Mcp,
            partner_id: PartnerId::tool(ToolId(1)),
            ..Default::default()
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
            ..Default::default()
        });

        let ledger = EvidenceLedger { traces: &[trace] };

        let mut workspace = MiningWorkspace::new();
        let candidates =
            CoevolutionEnhancer::generate_candidate_chunks(&config, &ledger, &mut workspace);

        assert!(!candidates.is_empty());
        let chunk = &candidates[0];
        assert_eq!(chunk.suggested_edge.from, NodeId(0));
        assert_eq!(chunk.suggested_edge.to, NodeId(2));
        assert_eq!(chunk.bypass_node, NodeId(1));
        assert!(chunk.reason.contains("high-latency"));
    }
}
