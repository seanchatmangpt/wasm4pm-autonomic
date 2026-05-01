use insa_types::{FieldMask, NodeId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimdExecutor;

impl SimdExecutor {
    pub fn execute_nodes(&self, nodes: &[NodeId]) -> FieldMask {
        let mut combined = 0u64;
        for node in nodes {
            combined |= node.0 as u64;
        }
        FieldMask(combined)
    }
}
