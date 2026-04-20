//! Branchless Implementation: tzmsk_u64
#[inline(always)]
#[no_mangle]
pub fn tzmsk_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (!val) & val.wrapping_sub(1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn tzmsk_u64_reference(val: u64, aux: u64) -> u64 {
        (!val) & (val.wrapping_sub(1))
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(tzmsk_u64_reference(val, aux), tzmsk_u64(val, aux));
        }
    }
}
