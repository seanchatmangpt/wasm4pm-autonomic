//! Branchless Implementation: misra_gries_add
#[inline(always)]
#[no_mangle]
pub fn misra_gries_add(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn misra_gries_add_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(misra_gries_add_reference(val, aux), misra_gries_add(val, aux));
        }
    }
}
