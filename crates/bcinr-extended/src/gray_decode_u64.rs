//! Branchless Implementation: gray_decode_u64
#[inline(always)]
#[no_mangle]
pub fn gray_decode_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let mut n = val;
    n ^= n >> 32;
    n ^= n >> 16;
    n ^= n >> 8;
    n ^= n >> 4;
    n ^= n >> 2;
    n ^= n >> 1;
    n
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn gray_decode_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(gray_decode_u64_reference(val, aux), gray_decode_u64(val, aux));
        }
    }
}
