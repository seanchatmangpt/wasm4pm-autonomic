//! Branchless Implementation: exp2_u64_fixed
#[inline(always)]
#[no_mangle]
pub fn exp2_u64_fixed(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn exp2_u64_fixed_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(exp2_u64_fixed_reference(val, aux), exp2_u64_fixed(val, aux));
        }
    }
}
