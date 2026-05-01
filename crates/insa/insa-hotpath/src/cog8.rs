use insa_types::{InstinctByte, NodeId};

#[derive(Debug, Clone)]
pub struct Cog8Executor {
    pub base_id: NodeId,
    pub instinct: InstinctByte,
}

impl Cog8Executor {
    pub fn new(base_id: NodeId, instinct: InstinctByte) -> Self {
        Self { base_id, instinct }
    }

    pub fn run(&self) -> bool {
        self.instinct.0 > 0
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(32))]
pub struct Cog8Row {
    pub data: [u8; 32],
}
