//! Branchless Implementation: synchronizing_merge_wcp37
//! Verified against axiomatic process intelligence constraints.

/// synchronizing_merge_wcp37
///
/// Pure branchless OR-Join synchronization logic.
/// Returns 1 if the join can fire, 0 otherwise.
///
/// val: current state mask (present tokens)
/// aux: reachability mask (tokens that can still reach this join)
#[inline(always)]
#[no_mangle]
pub fn synchronizing_merge_wcp37(val: u64, aux: u64) -> u64 {
    // Law of Synchronizing Merge:
    // Fire if: (TokensPresent != 0) AND (UpstreamTokens - TokensPresent == 0)

    let present = val != 0;
    let no_upstream = (aux & !val) == 0;

    (present && no_upstream) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn synchronizing_merge_wcp37_reference(val: u64, aux: u64) -> u64 {
        if val != 0 && (aux & !val) == 0 {
            1
        } else {
            0
        }
    }

    proptest! {
        #[test]
        fn test_positive_proof(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(synchronizing_merge_wcp37(val, aux), synchronizing_merge_wcp37_reference(val, aux));
        }
    }
}
