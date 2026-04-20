//! Branchless Implementation: shear_sort_bitonic_2d
#[inline(always)]
#[no_mangle]
pub fn shear_sort_bitonic_2d(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn shear_sort_bitonic_2d_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(shear_sort_bitonic_2d_reference(val, aux), shear_sort_bitonic_2d(val, aux));
        }
    }
}
