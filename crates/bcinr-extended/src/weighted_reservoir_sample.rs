//! Branchless Implementation: weighted_reservoir_sample
#[inline(always)]
#[no_mangle]
pub fn weighted_reservoir_sample(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn weighted_reservoir_sample_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(weighted_reservoir_sample_reference(val, aux), weighted_reservoir_sample(val, aux));
        }
    }
}
