//! Branchless Implementation: log2_u64_fixed
#[inline(always)]
#[no_mangle]
pub fn log2_u64_fixed(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (63 - val.leading_zeros() as u64) << 16
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn log2_u64_fixed_reference(val: u64, aux: u64) -> u64 {
        ((val as f64).log2() * 65536.0) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(log2_u64_fixed_reference(val, aux), log2_u64_fixed(val, aux));
        }
    }
}
