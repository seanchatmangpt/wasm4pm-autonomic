//! Branchless Implementation: prefix_sum_simd_u32x8
#[inline(always)]
#[no_mangle]
pub fn prefix_sum_simd_u32x8(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn prefix_sum_simd_u32x8_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(prefix_sum_simd_u32x8_reference(val, aux), prefix_sum_simd_u32x8(val, aux));
        }
    }
}
