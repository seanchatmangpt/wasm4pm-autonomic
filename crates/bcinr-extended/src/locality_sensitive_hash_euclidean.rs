//! Branchless Implementation: locality_sensitive_hash_euclidean
#[inline(always)]
#[no_mangle]
pub fn locality_sensitive_hash_euclidean(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn locality_sensitive_hash_euclidean_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(locality_sensitive_hash_euclidean_reference(val, aux), locality_sensitive_hash_euclidean(val, aux));
        }
    }
}
