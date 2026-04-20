//! Branchless Implementation: parity_check_u128
#[inline(always)]
#[no_mangle]
pub fn parity_check_u128(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    ((val.count_ones() + aux.count_ones()) & 1) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn parity_check_u128_reference(val: u64, aux: u64) -> u64 {
        ((val.count_ones() + aux.count_ones()) & 1) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(parity_check_u128_reference(val, aux), parity_check_u128(val, aux));
        }
    }
}
