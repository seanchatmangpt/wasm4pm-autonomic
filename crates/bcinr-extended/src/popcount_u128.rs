//! Branchless Implementation: popcount_u128
#[inline(always)]
#[no_mangle]
pub fn popcount_u128(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val.count_ones() + aux.count_ones()) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn popcount_u128_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(popcount_u128_reference(val, aux), popcount_u128(val, aux));
        }
    }
}
