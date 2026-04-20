//! Branchless Implementation: abs_diff_i64
#[inline(always)]
#[no_mangle]
pub fn abs_diff_i64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val as i64).abs_diff(aux as i64)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn abs_diff_i64_reference(val: u64, aux: u64) -> u64 {
        (val as i64 as i128 - aux as i64 as i128).abs() as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(abs_diff_i64_reference(val, aux), abs_diff_i64(val, aux));
        }
    }
}
