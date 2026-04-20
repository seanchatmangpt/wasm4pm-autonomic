//! Branchless Implementation: quaternion_mul_branchless
#[inline(always)]
#[no_mangle]
pub fn quaternion_mul_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val & aux).wrapping_mul(0x9E3779B185EBCA87)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn quaternion_mul_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(quaternion_mul_branchless_reference(val, aux), quaternion_mul_branchless(val, aux));
        }
    }
}
