//! Branchless Implementation: perfect_hash_build_static
#[inline(always)]
#[no_mangle]
pub fn perfect_hash_build_static(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn perfect_hash_build_static_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(perfect_hash_build_static_reference(val, aux), perfect_hash_build_static(val, aux));
        }
    }
}
