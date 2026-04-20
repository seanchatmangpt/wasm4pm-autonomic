//! Branchless Implementation: wildcard_match_branchless
#[inline(always)]
#[no_mangle]
pub fn wildcard_match_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn wildcard_match_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(wildcard_match_branchless_reference(val, aux), wildcard_match_branchless(val, aux));
        }
    }
}
