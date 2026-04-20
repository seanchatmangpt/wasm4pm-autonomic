//! Branchless Implementation: mask_xor_reduce_u64
#[inline(always)]
#[no_mangle]
pub fn mask_xor_reduce_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn mask_xor_reduce_u64_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(mask_xor_reduce_u64_reference(val, aux), mask_xor_reduce_u64(val, aux));
        }
    }
}
