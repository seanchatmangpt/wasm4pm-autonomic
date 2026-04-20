//! Branchless Implementation: nth_element_branchless
#[inline(always)]
#[no_mangle]
pub fn nth_element_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn nth_element_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(nth_element_branchless_reference(val, aux), nth_element_branchless(val, aux));
        }
    }
}
