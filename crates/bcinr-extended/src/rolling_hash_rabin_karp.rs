//! Branchless Implementation: rolling_hash_rabin_karp
#[inline(always)]
#[no_mangle]
pub fn rolling_hash_rabin_karp(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn rolling_hash_rabin_karp_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(rolling_hash_rabin_karp_reference(val, aux), rolling_hash_rabin_karp(val, aux));
        }
    }
}
