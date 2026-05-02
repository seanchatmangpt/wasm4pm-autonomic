import os

def write_file(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(content)

cog8_content = """//! COG8 semantic closure substrate.
//!
//! Provides the core data structures for evaluating closed field context
//! into instinctual activation.

use insa_types::{PackId, GroupId, RuleId, BreedId, CompletedMask, FieldMask};
use insa_instinct::{InstinctByte, SelectedInstinctByte};
use insa_kappa8::KappaByte;
use insa_types::{NodeId, EdgeId};

/// Determines how the result of a cognitive node is projected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum CollapseFn {
    /// Result is a verifiable boolean check.
    Predicate = 0,
    /// Result is an expert rule conclusion.
    ExpertRule = 1,
    /// Result is an external projection.
    Projection = 2,
    /// Result requires HITL intervention.
    HumanInTheLoop = 3,
}

impl Default for CollapseFn {
    fn default() -> Self {
        Self::Predicate
    }
}

/// A single atomic closure evaluation row.
///
/// Designed to be exactly 32 bytes to ensure cache-line density 
/// (two rows per L1 cache line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C, align(32))]
pub struct Cog8Row {
    /// Bitmask of required fields.
    pub required_mask: FieldMask,
    /// Bitmask of forbidden fields.
    pub forbidden_mask: FieldMask,
    /// Predecessor nodes that must have completed.
    pub predecessor_mask: CompletedMask,
    
    /// Pack ID this row belongs to.
    pub pack_id: PackId,
    /// Group ID this row belongs to.
    pub group_id: GroupId,
    /// Rule ID this row belongs to.
    pub rule_id: RuleId,
    /// Breed ID defining the execution semantics.
    pub breed_id: BreedId,

    /// How the result collapses into the state.
    pub collapse_fn: CollapseFn,
    /// Execution priority when multiple rows match.
    pub priority: u8,
    
    /// Instinct Activation triggered by a successful match.
    pub response: InstinctByte,
    
    /// Cognitive Attribution identifying why this closure became actionable.
    pub kappa: KappaByte,
    
    /// Unused padding to reach 32 bytes exactly. Public for Default struct initialization in tests.
    pub _padding: [u8; 4],
}

/// The operator for a process motion edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Powl8Op {
    /// Terminal state reached.
    NoOp = 0,
    /// Execute the connected closure row.
    Act = 1,
}

impl Default for Powl8Op {
    fn default() -> Self {
        Self::NoOp
    }
}

/// The topological structure of the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EdgeKind {
    /// Unconditional transition.
    Sequence = 0,
    /// Exclusive choice branch.
    Choice = 1,
    /// Parallel execution branch.
    Parallel = 2,
    /// Required join condition.
    Join = 3,
    /// Re-evaluating cycle.
    Loop = 4,
}

impl Default for EdgeKind {
    fn default() -> Self {
        Self::Sequence
    }
}

/// The instruction payload carried by an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Powl8Instr {
    /// The operation to perform.
    pub op: Powl8Op,
    /// The collapse function to apply on the result.
    pub collapse_fn: CollapseFn,
    /// The target node id.
    pub node_id: NodeId,
    /// The edge identifier.
    pub edge_id: EdgeId,
    /// The mask of completion bits required to traverse this edge.
    pub guard_mask: CompletedMask,
    /// The mask of completion bits applied upon traversing this edge.
    pub effect_mask: CompletedMask,
}

/// A directed edge in the autonomic process topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cog8Edge {
    /// Source node id.
    pub from: NodeId,
    /// Destination node id.
    pub to: NodeId,
    /// Topological kind.
    pub kind: EdgeKind,
    /// Payload instruction.
    pub instr: Powl8Instr,
}

/// Decision output from the COG8 graph executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cog8Decision {
    /// The highest priority instinct resolved.
    pub response: InstinctByte,
    /// The cognitive attribution of the resolved decision.
    pub kappa: KappaByte,
    /// The mask of rows that successfully matched.
    pub fired_mask: u64,
    /// The mask of rows that failed to match.
    pub denied_mask: u64,
    /// The updated completion state of the graph.
    pub completed_mask: u64,
    /// Pack ID of the highest priority matched row.
    pub matched_pack_id: Option<PackId>,
    /// Group ID of the highest priority matched row.
    pub matched_group_id: Option<GroupId>,
    /// Rule ID of the highest priority matched row.
    pub matched_rule_id: Option<RuleId>,
    /// Breed ID of the highest priority matched row.
    pub matched_breed_id: Option<BreedId>,
    /// Collapse Function of the highest priority matched row.
    pub collapse_fn: Option<CollapseFn>,
    /// Node ID selected for execution.
    pub selected_node: Option<NodeId>,
    /// Edge ID traversed to reach the selected node.
    pub selected_edge: Option<EdgeId>,
}

impl Default for Cog8Decision {
    fn default() -> Self {
        Self {
            response: InstinctByte::IGNORE,
            kappa: KappaByte::default(),
            fired_mask: 0,
            denied_mask: 0,
            completed_mask: 0,
            matched_pack_id: None,
            matched_group_id: None,
            matched_rule_id: None,
            matched_breed_id: None,
            collapse_fn: None,
            selected_node: None,
            selected_edge: None,
        }
    }
}

/// Bounded graph executor for nonlinear COG8 topologies.
///
/// Walks edges, evaluates ready nodes, and collapses the field state into a
/// single canonical response with full attribution.
#[inline(always)]
pub fn execute_cog8_graph(
    nodes: &[Cog8Row],
    edges: &[Cog8Edge],
    present: u64,
    mut completed: u64,
) -> Result<Cog8Decision, &'static str> {
    let mut best = Cog8Decision {
        response: InstinctByte::IGNORE,
        completed_mask: completed,
        ..Default::default()
    };

    let mut best_priority = 0u8;
    let num_nodes = nodes.len();

    for edge in edges {
        let instr = &edge.instr;

        // Guard check: are the prerequisite completion bits set?
        if (completed & instr.guard_mask.0) == instr.guard_mask.0 {
            let node_index = edge.to.0 as usize;

            if node_index < num_nodes {
                let row = &nodes[node_index];

                // Optimized bitwise match evaluation
                let m1 = (present & row.required_mask.0) ^ row.required_mask.0;
                let m2 = present & row.forbidden_mask.0;
                let m3 = (completed & row.predecessor_mask.0) ^ row.predecessor_mask.0;
                let matched = (m1 | m2 | m3) == 0;

                // Both matched and !matched advance completed
                completed |= instr.effect_mask.0;

                // Branchless mask updates
                let matched_bit = (matched as u64) << (node_index % 64);
                let denied_bit = ((!matched) as u64) << (node_index % 64);
                best.fired_mask |= matched_bit;
                best.denied_mask |= denied_bit;

                // Attribution: capture the highest-priority closure.
                if matched && row.priority >= best_priority {
                    best_priority = row.priority;
                    best.response = row.response;
                    best.kappa = row.kappa;
                    best.matched_pack_id = Some(row.pack_id);
                    best.matched_group_id = Some(row.group_id);
                    best.matched_rule_id = Some(row.rule_id);
                    best.matched_breed_id = Some(row.breed_id);
                    best.collapse_fn = Some(row.collapse_fn);
                    best.selected_node = Some(edge.to);
                    best.selected_edge = Some(instr.edge_id);
                }
            } else {
                return Err("Node out of bounds");
            }
        }
    }

    best.completed_mask = completed;
    Ok(best)
}
"""

powl8_content = """//! POWL8 Autonomic Motion execution layer.
//!
//! Provides the engine for moving resolved instincts through the topology.

use insa_types::{NodeId, CompletedMask};
use insa_instinct::{InstinctByte, SelectedInstinctByte};
use insa_kappa8::KappaByte;
use crate::cog8::{Cog8Edge, Powl8Instr};

/// The outcome of an instinct resolution cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutonomicMotion {
    pub fired_node: NodeId,
    pub selected_instinct: SelectedInstinctByte,
    pub instruction: Powl8Instr,
}
"""

write_file('../insa/insa-hotpath/src/cog8.rs', cog8_content)
write_file('../insa/insa-hotpath/src/powl8.rs', powl8_content)
print("Files written successfully.")
