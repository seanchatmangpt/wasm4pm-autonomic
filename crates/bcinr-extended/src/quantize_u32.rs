//! Branchless Implementation: quantize_u32
#[inline(always)]
#[no_mangle]
pub fn quantize_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val / (aux + (aux == 0) as u64)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn quantize_u32_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(quantize_u32_reference(val, aux), quantize_u32(val, aux));
        }
    }
}
