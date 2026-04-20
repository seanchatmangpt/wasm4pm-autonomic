//! Branchless Implementation: add_sat_i32
#[inline(always)]
#[no_mangle]
pub fn add_sat_i32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let res = (val as i32).wrapping_add(aux as i32);
    let overflow = ((val as i32 ^ res) & (aux as i32 ^ res)) >> 31;
    let sat = (val as i32 >> 31) ^ i32::MAX;
    ((res & !overflow) | (sat & overflow)) as u32 as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn add_sat_i32_reference(val: u64, aux: u64) -> u64 {
        (val as i32).saturating_add(aux as i32) as u32 as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(add_sat_i32_reference(val, aux), add_sat_i32(val, aux));
        }
    }
}
