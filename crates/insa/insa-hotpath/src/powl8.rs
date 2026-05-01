//! POWL8 Autonomic Motion execution layer.

use insa_instinct::SelectedInstinctByte;
use insa_types::{CompletedMask, EdgeId, NodeId, Powl8Op};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Powl8Instr {
    pub op: Powl8Op,
    pub node_id: NodeId,
    pub edge_id: EdgeId,
    pub guard_mask: CompletedMask,
    pub effect_mask: CompletedMask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cog8Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub instr: Powl8Instr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutonomicMotion {
    pub fired_node: NodeId,
    pub selected_instinct: SelectedInstinctByte,
    pub instruction: Powl8Instr,
}
