//! Branchless Implementation: bsd_checksum_u16
#[inline(always)]
#[no_mangle]
pub fn bsd_checksum_u16(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bsd_checksum_u16_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bsd_checksum_u16_reference(val, aux), bsd_checksum_u16(val, aux));
        }
    }
}
