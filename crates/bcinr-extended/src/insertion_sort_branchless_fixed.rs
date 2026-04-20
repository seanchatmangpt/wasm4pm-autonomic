//! Branchless Implementation: insertion_sort_branchless_fixed
#[inline(always)]
#[no_mangle]
pub fn insertion_sort_branchless_fixed(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn insertion_sort_branchless_fixed_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(insertion_sort_branchless_fixed_reference(val, aux), insertion_sort_branchless_fixed(val, aux));
        }
    }
}
