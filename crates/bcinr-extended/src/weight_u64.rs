//! Branchless Implementation: weight_u64
#[inline(always)]
#[no_mangle]
pub fn weight_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.count_ones() as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn weight_u64_reference(val: u64, aux: u64) -> u64 {
        val.count_ones() as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(weight_u64_reference(val, aux), weight_u64(val, aux));
        }
    }
}
