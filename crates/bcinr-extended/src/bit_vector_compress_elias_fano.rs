//! Branchless Implementation: bit_vector_compress_elias_fano
#[inline(always)]
#[no_mangle]
pub fn bit_vector_compress_elias_fano(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bit_vector_compress_elias_fano_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bit_vector_compress_elias_fano_reference(val, aux), bit_vector_compress_elias_fano(val, aux));
        }
    }
}
