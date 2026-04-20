//! Branchless Implementation: mul_sat_u64
#[inline(always)]
#[no_mangle]
pub fn mul_sat_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let (res, overflow) = val.overflowing_mul(aux);
    res | (0u64.wrapping_sub(overflow as u64))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn mul_sat_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(mul_sat_u64_reference(val, aux), mul_sat_u64(val, aux));
        }
    }
}
