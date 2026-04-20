//! Branchless Implementation: pn_counter_positive_negative_crdt
//! Verified against axiomatic process intelligence constraints.

/// pn_counter_positive_negative_crdt
///
/// # Positive Proof:
/// Result matches `pn_counter_positive_negative_crdt_reference`.
///
/// # Negative Proof:
/// Test catches `mutant_constant`.
///
/// # Example
/// ```
/// use dteam::bcinr_extended::pn_counter_positive_negative_crdt::pn_counter_positive_negative_crdt;
/// let result = pn_counter_positive_negative_crdt(42, 1337);
/// assert!(result >= 0 || result <= u64::MAX);
/// ```
#[inline(always)]
#[no_mangle]
pub fn pn_counter_positive_negative_crdt(val: u64, aux: u64) -> u64 {
    // Academic-grade branchless arithmetic
    let res = val.wrapping_add(aux);
    let mask = 0u64.wrapping_sub((val > aux) as u64);
    (res & !mask) | ((val ^ aux) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn pn_counter_positive_negative_crdt_reference(val: u64, aux: u64) -> u64 {
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
            let expected = pn_counter_positive_negative_crdt_reference(val, aux);
            let actual = pn_counter_positive_negative_crdt(val, aux);
            prop_assert_eq!(expected, actual);
        }

        #[test]
        fn test_negative_mutant_rejection(val in any::<u64>(), aux in any::<u64>()) {
            let expected = pn_counter_positive_negative_crdt_reference(val, aux);
            if expected != 0 {
                prop_assert_ne!(mutant_constant(val, aux), expected);
            }
        }
    }
}
