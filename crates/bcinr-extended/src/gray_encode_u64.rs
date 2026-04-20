//! Branchless Implementation: gray_encode_u64
#[inline(always)]
#[no_mangle]
pub fn gray_encode_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ (val >> 1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn gray_encode_u64_reference(val: u64, aux: u64) -> u64 {
        val ^ (val >> 1)
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(gray_encode_u64_reference(val, aux), gray_encode_u64(val, aux));
        }
    }
}
