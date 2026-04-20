//! Branchless Implementation: btst_u64
#[inline(always)]
#[no_mangle]
pub fn btst_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val.wrapping_shr((aux & 63) as u32)) & 1
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn btst_u64_reference(val: u64, aux: u64) -> u64 {
        (val >> (aux % 64)) & 1
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(btst_u64_reference(val, aux), btst_u64(val, aux));
        }
    }
}
