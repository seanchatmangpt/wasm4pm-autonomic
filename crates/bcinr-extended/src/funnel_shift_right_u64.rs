//! Branchless Implementation: funnel_shift_right_u64
#[inline(always)]
#[no_mangle]
pub fn funnel_shift_right_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let shift = (aux & 0x3F) as u32;
    (aux.wrapping_shr(shift))
        | (val.wrapping_shl((64u32.wrapping_sub(shift)) & 0x3F)
            & (0u64.wrapping_sub((shift != 0) as u64)))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn funnel_shift_right_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(funnel_shift_right_u64_reference(val, aux), funnel_shift_right_u64(val, aux));
        }
    }
}
