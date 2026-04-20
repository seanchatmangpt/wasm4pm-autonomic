//! Branchless Implementation: succinct_bit_vector_select
#[inline(always)]
#[no_mangle]
pub fn succinct_bit_vector_select(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn succinct_bit_vector_select_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(succinct_bit_vector_select_reference(val, aux), succinct_bit_vector_select(val, aux));
        }
    }
}
