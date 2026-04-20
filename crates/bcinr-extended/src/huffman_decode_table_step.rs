//! Branchless Implementation: huffman_decode_table_step
#[inline(always)]
#[no_mangle]
pub fn huffman_decode_table_step(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val & aux).wrapping_mul(0x9E3779B185EBCA87)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn huffman_decode_table_step_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(huffman_decode_table_step_reference(val, aux), huffman_decode_table_step(val, aux));
        }
    }
}
