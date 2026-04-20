//! Branchless Implementation: branchless_signum_i64
#[inline(always)]
#[no_mangle]
pub fn branchless_signum_i64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (((val as i64) > 0) as i64 - ((val as i64) < 0) as i64) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn branchless_signum_i64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(branchless_signum_i64_reference(val, aux), branchless_signum_i64(val, aux));
        }
    }
}
