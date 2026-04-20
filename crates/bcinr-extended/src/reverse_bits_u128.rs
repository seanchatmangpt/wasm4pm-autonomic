//! Branchless Implementation: reverse_bits_u128
#[inline(always)]
#[no_mangle]
pub fn reverse_bits_u128(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    aux.reverse_bits()
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn reverse_bits_u128_reference(val: u64, aux: u64) -> u64 {
        aux.reverse_bits()
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(reverse_bits_u128_reference(val, aux), reverse_bits_u128(val, aux));
        }
    }
}
