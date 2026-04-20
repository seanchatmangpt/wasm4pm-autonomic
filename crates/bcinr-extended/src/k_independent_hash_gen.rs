//! Branchless Implementation: k_independent_hash_gen
#[inline(always)]
#[no_mangle]
pub fn k_independent_hash_gen(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn k_independent_hash_gen_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(k_independent_hash_gen_reference(val, aux), k_independent_hash_gen(val, aux));
        }
    }
}
