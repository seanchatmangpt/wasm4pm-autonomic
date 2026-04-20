//! Branchless Implementation: blsi_u64
#[inline(always)]
#[no_mangle]
pub fn blsi_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val & val.wrapping_neg()
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn blsi_u64_reference(val: u64, aux: u64) -> u64 {
        if val == 0 {
            0
        } else {
            val & (0u64.wrapping_sub(val))
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(blsi_u64_reference(val, aux), blsi_u64(val, aux));
        }
    }
}
