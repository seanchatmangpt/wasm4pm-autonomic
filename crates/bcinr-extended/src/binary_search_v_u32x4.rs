//! Branchless Implementation: binary_search_v_u32x4
#[inline(always)]
#[no_mangle]
pub fn binary_search_v_u32x4(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn binary_search_v_u32x4_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(binary_search_v_u32x4_reference(val, aux), binary_search_v_u32x4(val, aux));
        }
    }
}
