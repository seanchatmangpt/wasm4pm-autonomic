use insa_types::{InstinctByte, KappaByte};

#[derive(Debug, Clone)]
pub struct InstinctResolutionLut {
    pub table: [KappaByte; 256],
}

impl InstinctResolutionLut {
    pub fn new(table: [KappaByte; 256]) -> Self {
        Self { table }
    }

    pub fn resolve(&self, byte: InstinctByte) -> KappaByte {
        self.table[byte.0 as usize]
    }
}
