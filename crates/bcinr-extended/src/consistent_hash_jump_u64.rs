//! Branchless Implementation: consistent_hash_jump_u64
#[inline(always)]
#[no_mangle]
pub fn consistent_hash_jump_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn consistent_hash_jump_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(consistent_hash_jump_u64_reference(val, aux), consistent_hash_jump_u64(val, aux));
        }
    }
}
