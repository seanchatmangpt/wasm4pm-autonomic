#[derive(Debug, Clone)]
pub struct ExecutionReceipt {
    pub success: bool,
    pub gas_used: u64,
}

impl ExecutionReceipt {
    pub fn new(success: bool, gas_used: u64) -> Self {
        Self { success, gas_used }
    }
}
