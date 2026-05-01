use crate::receipt::ExecutionReceipt;

#[derive(Debug, Default)]
pub struct ReplayEngine {
    pub history: Vec<ExecutionReceipt>,
}

impl ReplayEngine {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }

    pub fn record(&mut self, receipt: ExecutionReceipt) {
        self.history.push(receipt);
    }
}
