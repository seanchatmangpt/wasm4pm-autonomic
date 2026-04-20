//! Branchless Implementation: partial_sort_branchless_k
#[inline(always)]
#[no_mangle]
pub fn partial_sort_branchless_k(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn partial_sort_branchless_k_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(partial_sort_branchless_k_reference(val, aux), partial_sort_branchless_k(val, aux));
        }
    }
}
