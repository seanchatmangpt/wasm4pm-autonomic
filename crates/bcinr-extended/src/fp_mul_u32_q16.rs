//! Branchless Implementation: fp_mul_u32_q16
#[inline(always)]
#[no_mangle]
pub fn fp_mul_u32_q16(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val.wrapping_mul(aux)) >> 16
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn fp_mul_u32_q16_reference(val: u64, aux: u64) -> u64 {
        ((val as u128 * aux as u128) >> 16) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(fp_mul_u32_q16_reference(val, aux), fp_mul_u32_q16(val, aux));
        }
    }
}
