//! Branchless Implementation: green_sorting_network_16
#[inline(always)]
#[no_mangle]
pub fn green_sorting_network_16(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn green_sorting_network_16_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(green_sorting_network_16_reference(val, aux), green_sorting_network_16(val, aux));
        }
    }
}
