//! COG8 bounded cognitive closure nodes and POWL8 ISA executor (PRD v0.4).
//!
//! This module implements the "Stable Rust Zero-Cost Runtime" for nonlinear
//! cognitive graphs. Every JTBD is decomposed into a graph of COG8 nodes
//! (≤8 load-bearing variables) connected by POWL topology (choice graphs,
//! partial orders, loops).

pub use crate::ids::*;

// =============================================================================
// Stable Rust Core Data Model
// =============================================================================

/// Canonical response class.
#[repr(u8)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Instinct {
    /// Known harmless event — return to baseline.
    Settle = 0,
    /// Expected package/delivery — retrieve now.
    Retrieve = 1,
    /// Unknown but low-threat — inspect.
    Inspect = 2,
    /// Missing evidence — request clarification.
    Ask = 3,
    /// Action does not belong — refuse the transition.
    Refuse = 4,
    /// Persistent unresolved disturbance — escalate.
    Escalate = 5,
    /// No-op. Default — safest fallback when no other variant applies.
    #[default]
    Ignore = 6,
}

impl From<crate::instinct::AutonomicInstinct> for Instinct {
    fn from(old: crate::instinct::AutonomicInstinct) -> Self {
        use crate::instinct::AutonomicInstinct::*;
        match old {
            Settle => Self::Settle,
            Retrieve => Self::Retrieve,
            Inspect => Self::Inspect,
            Ask => Self::Ask,
            Refuse => Self::Refuse,
            Escalate => Self::Escalate,
            Ignore => Self::Ignore,
        }
    }
}

/// POWL8 local instruction vocabulary.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Powl8Op {
    /// Evaluate a COG8 closure node.
    Act = 0,
    /// Traverse an admissible choice-graph edge.
    Choice = 1,
    /// Record an independent closure as satisfied.
    Partial = 2,
    /// Combine satisfied closures into higher closure.
    Join = 3,
    /// Perform bounded re-evaluation.
    Loop = 4,
    /// Move internally without emitted response.
    #[default]
    Silent = 5,
    /// Block invalid route/action.
    Block = 6,
    /// Emit canonical runtime response.
    Emit = 7,
}

/// POWL8 Instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Powl8Instr {
    /// Instruction operation.
    pub op: Powl8Op,
    /// Collapse function attributed to this instruction.
    pub collapse_fn: CollapseFn,
    /// Target node ID.
    pub node_id: NodeId,
    /// Associated edge ID.
    pub edge_id: EdgeId,
    /// Predecessor completion mask required to run.
    pub guard_mask: u64,
    /// Completion bits set after successful run.
    pub effect_mask: u64,
}

/// One bounded cognitive closure node (COG8).
///
/// Law: No single runtime cognitive operator may bind more than 8 load-bearing
/// closure variables.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Cog8Row {
    /// Source pack.
    pub pack_id: PackId,
    /// Precedence group.
    pub group_id: GroupId,
    /// Stable rule ID.
    pub rule_id: RuleId,
    /// Admitted reasoning breed.
    pub breed_id: BreedId,
    /// Collapse function.
    pub collapse_fn: CollapseFn,

    /// Up to 8 load-bearing variable IDs.
    pub var_ids: [FieldId; 8],
    /// Mask of variables that MUST be present.
    pub required_mask: u64,
    /// Mask of variables that MUST NOT be present.
    pub forbidden_mask: u64,
    /// Completion bits required to fire.
    pub predecessor_mask: u64,

    /// Response admitted by this closure.
    pub response: Instinct,
    /// Priority for tie-breaking.
    pub priority: u16,
}

/// Topology edge kind.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum EdgeKind {
    /// Non-block-structured branch.
    Choice = 1,
    /// Independent closure satisfy.
    PartialOrder = 2,
    /// Bounded re-evaluation.
    Loop = 3,
    /// Internal transition.
    Silent = 4,
    /// High-priority replacement.
    Override = 5,
    /// Path invalidation.
    Blocking = 6,
    /// No relation.
    #[default]
    None = 0,
}

/// Topology edge connecting COG8 nodes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Cog8Edge {
    /// Source node index.
    pub from: NodeId,
    /// Target node index.
    pub to: NodeId,
    /// Topological relation.
    pub kind: EdgeKind,
    /// Instruction to execute.
    pub instr: Powl8Instr,
}

/// Result of a COG8 graph execution.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Cog8Decision {
    /// Selected response.
    pub response: Instinct,
    /// Pack provenance.
    pub matched_pack_id: Option<PackId>,
    /// Group provenance.
    pub matched_group_id: Option<GroupId>,
    /// Rule provenance.
    pub matched_rule_id: Option<RuleId>,
    /// Breed provenance.
    pub matched_breed_id: Option<BreedId>,
    /// Collapse attribution.
    pub collapse_fn: Option<CollapseFn>,
    /// Graph traversal provenance.
    pub selected_node: Option<NodeId>,
    /// Edge traversal provenance.
    pub selected_edge: Option<EdgeId>,
    /// Mask of all nodes reached and processed (advanced).
    pub completed_mask: u64,
    /// Mask of all nodes that fired successfully.
    pub fired_mask: u64,
    /// Mask of all nodes that were reached but denied by requirement match.
    pub denied_mask: u64,
}

/// Runtime-loaded field pack artifact (PRD v0.4 Bridge).
///
/// Preserves the metadata and ID mappings for a compiled pack.
#[derive(Clone, Debug)]
pub struct LoadedFieldPack {
    /// Numeric ID assigned to this pack.
    pub id: PackId,
    /// Canonical name (for reports).
    pub name: &'static str,
    /// Ontology profile.
    pub ontology_profile: Vec<String>,
    /// Pack digest URN.
    pub digest_urn: String,
}

// =============================================================================
// Stable Rust COG8 Matching & Execution
// =============================================================================

/// True iff the COG8 row requirements are satisfied by the present and completed masks.
///
/// # Examples
///
/// ```
/// use ccog::runtime::cog8::{cog8_matches, Cog8Row, Instinct};
/// use ccog::ids::*;
///
/// let row = Cog8Row {
///     required_mask: 0b01,
///     forbidden_mask: 0b10,
///     predecessor_mask: 0b100,
///     ..Cog8Row::default()
/// };
///
/// // Matches if required is present, forbidden is absent, and predecessor is completed.
/// assert!(cog8_matches(&row, 0b01, 0b100));
///
/// // Fails if required is missing.
/// assert!(!cog8_matches(&row, 0b00, 0b100));
///
/// // Fails if forbidden is present.
/// assert!(!cog8_matches(&row, 0b11, 0b100));
///
/// // Fails if predecessor is not completed.
/// assert!(!cog8_matches(&row, 0b01, 0b000));
/// ```
#[inline(always)]
pub fn cog8_matches(row: &Cog8Row, present: u64, completed: u64) -> bool {
    (present & row.required_mask) == row.required_mask
        && (present & row.forbidden_mask) == 0
        && (completed & row.predecessor_mask) == row.predecessor_mask
}

/// Bounded graph executor for nonlinear COG8 topologies.
///
/// Walks edges, evaluates ready nodes, and collapses the field state into a
/// single canonical response with full attribution.
///
/// This core function takes a raw `present` mask for testability and
/// performance in batch dispatch.
#[inline(always)]
pub fn execute_cog8_graph(
    nodes: &[Cog8Row],
    edges: &[Cog8Edge],
    present: u64,
    mut completed: u64,
) -> crate::runtime::error::Result<Cog8Decision> {
    let mut best = Cog8Decision {
        response: Instinct::Ignore,
        matched_pack_id: None,
        matched_group_id: None,
        matched_rule_id: None,
        matched_breed_id: None,
        collapse_fn: None,
        selected_node: None,
        selected_edge: None,
        completed_mask: completed,
        fired_mask: 0,
        denied_mask: 0,
    };

    let mut best_priority = 0u16;
    let num_nodes = nodes.len();

    for edge in edges {
        let instr = &edge.instr;

        // Guard check: are the prerequisite completion bits set?
        if (completed & instr.guard_mask) == instr.guard_mask {
            let node_index = edge.to.0 as usize;

            if node_index < num_nodes {
                let row = &nodes[node_index];

                // Optimized bitwise match evaluation: reduce 3 branches to 1.
                let m1 = (present & row.required_mask) ^ row.required_mask;
                let m2 = present & row.forbidden_mask;
                let m3 = (completed & row.predecessor_mask) ^ row.predecessor_mask;
                let matched = (m1 | m2 | m3) == 0;

                // Both matched and !matched advance completed
                completed |= instr.effect_mask;

                // Branchless mask updates
                let matched_bit = (matched as u64) << (node_index % 64);
                let denied_bit = ((!matched) as u64) << (node_index % 64);
                best.fired_mask |= matched_bit;
                best.denied_mask |= denied_bit;

                // Attribution: capture the highest-priority closure.
                if matched && row.priority >= best_priority {
                    best_priority = row.priority;
                    best.response = row.response;
                    best.matched_pack_id = Some(row.pack_id);
                    best.matched_group_id = Some(row.group_id);
                    best.matched_rule_id = Some(row.rule_id);
                    best.matched_breed_id = Some(row.breed_id);
                    best.collapse_fn = Some(row.collapse_fn);
                    best.selected_node = Some(edge.to);
                    best.selected_edge = Some(instr.edge_id);
                }
            } else {
                return Err(crate::runtime::error::RuntimeError::NodeOutOfBounds {
                    index: node_index,
                    max: num_nodes,
                    edge_id: instr.edge_id,
                });
            }
        }
    }

    best.completed_mask = completed;
    Ok(best)
}

/// High-level wrapper for [`execute_cog8_graph`] that derives the present mask
/// from the context.
#[inline(always)]
pub fn execute_cog8(
    nodes: &[Cog8Row],
    edges: &[Cog8Edge],
    context: &crate::runtime::ClosedFieldContext,
    completed: u64,
) -> crate::runtime::error::Result<Cog8Decision> {
    let present = crate::compiled_hook::compute_present_mask(&context.snapshot);
    execute_cog8_graph(nodes, edges, present, completed)
}
