//! Branchless Implementation: fp_div_u32_q16
#[inline(always)]
#[no_mangle]
pub fn fp_div_u32_q16(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val << 16) / (aux + (aux == 0) as u64)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn fp_div_u32_q16_reference(val: u64, aux: u64) -> u64 {
        (((val as u128) << 16) / (aux as u128)) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(fp_div_u32_q16_reference(val, aux), fp_div_u32_q16(val, aux));
        }
    }
}
