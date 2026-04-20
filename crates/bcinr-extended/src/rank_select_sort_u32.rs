//! Branchless Implementation: rank_select_sort_u32
#[inline(always)]
#[no_mangle]
pub fn rank_select_sort_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn rank_select_sort_u32_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(rank_select_sort_u32_reference(val, aux), rank_select_sort_u32(val, aux));
        }
    }
}
