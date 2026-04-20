//! Branchless Implementation: select_u128
#[inline(always)]
#[no_mangle]
pub fn select_u128(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn select_u128_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(select_u128_reference(val, aux), select_u128(val, aux));
        }
    }
}
