//! Branchless Implementation: norm_u32
#[inline(always)]
#[no_mangle]
pub fn norm_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn norm_u32_reference(val: u64, aux: u64) -> u64 {
        (((val & 0xFFFFFFFF) as f64).powi(2) + ((val >> 32) as f64).powi(2)).sqrt() as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(norm_u32_reference(val, aux), norm_u32(val, aux));
        }
    }
}
