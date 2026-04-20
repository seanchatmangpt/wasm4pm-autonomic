//! Branchless Implementation: is_prime_u64_branchless
#[inline(always)]
#[no_mangle]
pub fn is_prime_u64_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (!val.is_multiple_of(2)) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn is_prime_u64_branchless_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(is_prime_u64_branchless_reference(val, aux), is_prime_u64_branchless(val, aux));
        }
    }
}
