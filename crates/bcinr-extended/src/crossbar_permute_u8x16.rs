//! Branchless Implementation: crossbar_permute_u8x16
#[inline(always)]
#[no_mangle]
pub fn crossbar_permute_u8x16(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn crossbar_permute_u8x16_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(crossbar_permute_u8x16_reference(val, aux), crossbar_permute_u8x16(val, aux));
        }
    }
}
