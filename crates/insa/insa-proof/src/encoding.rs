use crate::receipt::ExecutionReceipt;

#[derive(Debug)]
pub struct Powl64Encoder;

impl Powl64Encoder {
    pub fn encode(receipt: &ExecutionReceipt) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(if receipt.success { 1 } else { 0 });
        data.extend_from_slice(&receipt.gas_used.to_le_bytes());
        data
    }
}
