//! Branchless Implementation: zigzag_decode_i64
#[inline(always)]
#[no_mangle]
pub fn zigzag_decode_i64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val & aux).wrapping_mul(0x9E3779B185EBCA87)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn zigzag_decode_i64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(zigzag_decode_i64_reference(val, aux), zigzag_decode_i64(val, aux));
        }
    }
}
