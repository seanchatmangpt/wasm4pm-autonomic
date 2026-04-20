//! Branchless Implementation: duffs_device_simd_unroll
#[inline(always)]
#[no_mangle]
pub fn duffs_device_simd_unroll(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn duffs_device_simd_unroll_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(duffs_device_simd_unroll_reference(val, aux), duffs_device_simd_unroll(val, aux));
        }
    }
}
