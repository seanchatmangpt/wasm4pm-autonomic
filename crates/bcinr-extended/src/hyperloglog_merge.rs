//! Branchless Implementation: hyperloglog_merge
#[inline(always)]
#[no_mangle]
pub fn hyperloglog_merge(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn hyperloglog_merge_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(hyperloglog_merge_reference(val, aux), hyperloglog_merge(val, aux));
        }
    }
}
