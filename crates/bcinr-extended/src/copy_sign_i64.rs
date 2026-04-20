//! Branchless Implementation: copy_sign_i64
#[inline(always)]
#[no_mangle]
pub fn copy_sign_i64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    ((val as i64).abs() * (aux as i64).signum()) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn copy_sign_i64_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(copy_sign_i64_reference(val, aux), copy_sign_i64(val, aux));
        }
    }
}
