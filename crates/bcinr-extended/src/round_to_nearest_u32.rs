//! Branchless Implementation: round_to_nearest_u32
#[inline(always)]
#[no_mangle]
pub fn round_to_nearest_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val + 32768) >> 16
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn round_to_nearest_u32_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(round_to_nearest_u32_reference(val, aux), round_to_nearest_u32(val, aux));
        }
    }
}
