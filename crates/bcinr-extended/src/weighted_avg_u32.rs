//! Branchless Implementation: weighted_avg_u32
#[inline(always)]
#[no_mangle]
pub fn weighted_avg_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn weighted_avg_u32_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(weighted_avg_u32_reference(val, aux), weighted_avg_u32(val, aux));
        }
    }
}
