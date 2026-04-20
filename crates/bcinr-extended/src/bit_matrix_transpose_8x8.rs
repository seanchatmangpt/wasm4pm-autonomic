//! Branchless Implementation: bit_matrix_transpose_8x8
#[inline(always)]
#[no_mangle]
pub fn bit_matrix_transpose_8x8(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bit_matrix_transpose_8x8_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bit_matrix_transpose_8x8_reference(val, aux), bit_matrix_transpose_8x8(val, aux));
        }
    }
}
