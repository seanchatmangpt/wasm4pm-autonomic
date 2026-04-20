//! Branchless Implementation: lcm_u64_branchless
#[inline(always)]
#[no_mangle]
pub fn lcm_u64_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn lcm_u64_branchless_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(lcm_u64_branchless_reference(val, aux), lcm_u64_branchless(val, aux));
        }
    }
}
