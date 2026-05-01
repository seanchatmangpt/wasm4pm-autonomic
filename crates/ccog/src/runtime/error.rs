use crate::ids::{EdgeId, NodeId};
use thiserror::Error;

/// Internal runtime errors with 'compiler-grade' diagnostics.
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// Loop budget exceeded during graph traversal.
    #[error("Loop Budget Exhausted at Node {node_id:?} after {iterations} iterations")]
    BudgetExhausted {
        /// The node where the budget was exhausted.
        node_id: NodeId,
        /// Number of iterations performed.
        iterations: u32,
    },

    /// Reference to a non-existent node in the graph.
    #[error("Node index out of bounds: {index} (max {max}) at Edge {edge_id:?}")]
    NodeOutOfBounds {
        /// The invalid index.
        index: usize,
        /// The maximum allowed index.
        max: usize,
        /// The edge that referenced the invalid node.
        edge_id: EdgeId,
    },

    /// Error during hook execution.
    #[error("Hook execution failed: {0}")]
    HookError(String),

    /// Error during graph state capture.
    #[error("Graph capture failed: {0}")]
    GraphError(String),

    /// Error during field materialization or ontology enforcement.
    #[error("Field error: {0}")]
    FieldError(String),

    /// MCP Transport or protocol error.
    #[error("MCP error: {0}")]
    McpError(String),

    /// A2A tasking or coordination error.
    #[error("A2A error: {0}")]
    A2AError(String),
}

/// Alias for Results using [`RuntimeError`].
pub type Result<T> = std::result::Result<T, RuntimeError>;
