//! Branchless Implementation: is_finite_fp32_branchless
#[inline(always)]
#[no_mangle]
pub fn is_finite_fp32_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    ((val >> 23) & 0xFF != 0xFF) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn is_finite_fp32_branchless_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(is_finite_fp32_branchless_reference(val, aux), is_finite_fp32_branchless(val, aux));
        }
    }
}
