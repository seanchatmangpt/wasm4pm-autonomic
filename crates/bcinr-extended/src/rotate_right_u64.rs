//! Branchless Implementation: rotate_right_u64
#[inline(always)]
#[no_mangle]
pub fn rotate_right_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.rotate_right((aux & 0x3F) as u32)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn rotate_right_u64_reference(val: u64, aux: u64) -> u64 {
        val.rotate_right((aux & 0x3F) as u32)
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(rotate_right_u64_reference(val, aux), rotate_right_u64(val, aux));
        }
    }
}
