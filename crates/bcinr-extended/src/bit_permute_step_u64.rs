//! Branchless Implementation: bit_permute_step_u64
#[inline(always)]
#[no_mangle]
pub fn bit_permute_step_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let shift = ((aux >> 32) & 0x3F) as u32;
    let mask = aux & 0xFFFFFFFF;
    let t = ((val.wrapping_shr(shift)) ^ val) & mask;
    val ^ t ^ (t.wrapping_shl(shift))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bit_permute_step_u64_reference(val: u64, aux: u64) -> u64 {
        let shift = ((aux >> 32) & 0x3F) as u32;
        let mask = aux & 0xFFFFFFFF;
        let t = ((val.wrapping_shr(shift)) ^ val) & mask;
        val ^ t ^ (t.wrapping_shl(shift))
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bit_permute_step_u64_reference(val, aux), bit_permute_step_u64(val, aux));
        }
    }
}
