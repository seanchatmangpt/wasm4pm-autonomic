//! Branchless Implementation: avg_ceil_u64
#[inline(always)]
#[no_mangle]
pub fn avg_ceil_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val | aux) - ((val ^ aux) >> 1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn avg_ceil_u64_reference(val: u64, aux: u64) -> u64 {
        (val as u128 + aux as u128).div_ceil(2) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(avg_ceil_u64_reference(val, aux), avg_ceil_u64(val, aux));
        }
    }
}
