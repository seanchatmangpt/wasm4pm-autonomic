//! Branchless Implementation: get_mask_boundary_low_u64
#[inline(always)]
#[no_mangle]
pub fn get_mask_boundary_low_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val & val.wrapping_neg()
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn get_mask_boundary_low_u64_reference(val: u64, aux: u64) -> u64 {
        val & val.wrapping_neg()
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(get_mask_boundary_low_u64_reference(val, aux), get_mask_boundary_low_u64(val, aux));
        }
    }
}
