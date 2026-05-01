use insa_types::{FieldMask, NodeId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarExecutor;

impl ScalarExecutor {
    pub fn execute_node(&self, node: NodeId) -> FieldMask {
        FieldMask(node.0 as u64)
    }
}
