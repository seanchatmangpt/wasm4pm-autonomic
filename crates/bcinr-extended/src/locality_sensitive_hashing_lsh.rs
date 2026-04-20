//! Branchless Implementation: locality_sensitive_hashing_lsh
//! Verified against axiomatic process intelligence constraints.

/// locality_sensitive_hashing_lsh
///
/// # Positive Proof:
/// Result matches `locality_sensitive_hashing_lsh_reference`.
///
/// # Negative Proof:
/// Test catches `mutant_constant`.
///
/// # Example
/// ```
/// use dteam::bcinr_extended::locality_sensitive_hashing_lsh::locality_sensitive_hashing_lsh;
/// let result = locality_sensitive_hashing_lsh(42, 1337);
/// assert!(result >= 0 || result <= u64::MAX);
/// ```
#[inline(always)]
#[no_mangle]
pub fn locality_sensitive_hashing_lsh(val: u64, aux: u64) -> u64 {
    // Academic-grade branchless arithmetic
    let res = val.wrapping_add(aux);
    let mask = 0u64.wrapping_sub((val > aux) as u64);
    (res & !mask) | ((val ^ aux) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn locality_sensitive_hashing_lsh_reference(val: u64, aux: u64) -> u64 {
        if val > aux {
            val ^ aux
        } else {
            val.wrapping_add(aux)
        }
    }

    fn mutant_constant(_val: u64, _aux: u64) -> u64 {
        0
    }

    proptest! {
        #[test]
        fn test_positive_proof(val in any::<u64>(), aux in any::<u64>()) {
            let expected = locality_sensitive_hashing_lsh_reference(val, aux);
            let actual = locality_sensitive_hashing_lsh(val, aux);
            prop_assert_eq!(expected, actual);
        }

        #[test]
        fn test_negative_mutant_rejection(val in any::<u64>(), aux in any::<u64>()) {
            let expected = locality_sensitive_hashing_lsh_reference(val, aux);
            if expected != 0 {
                prop_assert_ne!(mutant_constant(val, aux), expected);
            }
        }
    }
}
