//! Branchless Implementation: div_sat_u64
#[inline(always)]
#[no_mangle]
pub fn div_sat_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let mask = 0u64.wrapping_sub((aux == 0) as u64);
    (val.wrapping_div(aux + (aux == 0) as u64)) | mask
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn div_sat_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(div_sat_u64_reference(val, aux), div_sat_u64(val, aux));
        }
    }
}
