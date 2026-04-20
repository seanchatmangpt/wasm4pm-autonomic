//! Branchless Implementation: bool_slice_from_mask
#[inline(always)]
#[no_mangle]
pub fn bool_slice_from_mask(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bool_slice_from_mask_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bool_slice_from_mask_reference(val, aux), bool_slice_from_mask(val, aux));
        }
    }
}
