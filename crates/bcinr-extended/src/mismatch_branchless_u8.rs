//! Branchless Implementation: mismatch_branchless_u8
#[inline(always)]
#[no_mangle]
pub fn mismatch_branchless_u8(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn mismatch_branchless_u8_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(mismatch_branchless_u8_reference(val, aux), mismatch_branchless_u8(val, aux));
        }
    }
}
