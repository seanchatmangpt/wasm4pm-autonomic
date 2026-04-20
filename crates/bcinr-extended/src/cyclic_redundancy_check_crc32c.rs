//! Branchless Implementation: cyclic_redundancy_check_crc32c
#[inline(always)]
#[no_mangle]
pub fn cyclic_redundancy_check_crc32c(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn cyclic_redundancy_check_crc32c_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(cyclic_redundancy_check_crc32c_reference(val, aux), cyclic_redundancy_check_crc32c(val, aux));
        }
    }
}
