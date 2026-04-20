//! Branchless Implementation: fast_inverse_sqrt_u32
#[inline(always)]
#[no_mangle]
pub fn fast_inverse_sqrt_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let x = (val & 0xFFFFFFFF) as f32;
    let i = x.to_bits();
    let i = 0x5f3759df - (i >> 1);
    f32::from_bits(i) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn fast_inverse_sqrt_u32_reference(val: u64, aux: u64) -> u64 {
        (1.0 / (val as f32).sqrt()) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(fast_inverse_sqrt_u32_reference(val, aux), fast_inverse_sqrt_u32(val, aux));
        }
    }
}
