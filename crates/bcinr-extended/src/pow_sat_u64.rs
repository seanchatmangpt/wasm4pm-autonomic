//! Branchless Implementation: pow_sat_u64
#[inline(always)]
#[no_mangle]
pub fn pow_sat_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.saturating_pow(aux as u32)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn pow_sat_u64_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(pow_sat_u64_reference(val, aux), pow_sat_u64(val, aux));
        }
    }
}
