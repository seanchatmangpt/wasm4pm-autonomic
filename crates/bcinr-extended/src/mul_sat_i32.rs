//! Branchless Implementation: mul_sat_i32
#[inline(always)]
#[no_mangle]
pub fn mul_sat_i32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val as i32).saturating_mul(aux as i32) as u32 as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn mul_sat_i32_reference(val: u64, aux: u64) -> u64 {
        (val as i32).saturating_mul(aux as i32) as u32 as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(mul_sat_i32_reference(val, aux), mul_sat_i32(val, aux));
        }
    }
}
