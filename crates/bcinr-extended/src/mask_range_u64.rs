//! Branchless Implementation: mask_range_u64
#[inline(always)]
#[no_mangle]
pub fn mask_range_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let start = aux & 0x3F;
    let end = (aux >> 8) & 0x3F;
    let is_valid = (start < end) as u64;
    let diff = (end.wrapping_sub(start)) & 0x3F;
    let mask = (0u64.wrapping_sub((end.wrapping_sub(start) >= 64) as u64))
        | (((1u64.wrapping_shl(diff as u32)).wrapping_sub(1))
            & (0u64.wrapping_sub((end.wrapping_sub(start) < 64) as u64)));
    (mask.wrapping_shl(start as u32)) * is_valid
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn mask_range_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(mask_range_u64_reference(val, aux), mask_range_u64(val, aux));
        }
    }
}
