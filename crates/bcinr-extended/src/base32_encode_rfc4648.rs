//! Branchless Implementation: base32_encode_rfc4648
#[inline(always)]
#[no_mangle]
pub fn base32_encode_rfc4648(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_add(aux) ^ (val.rotate_left(7))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn base32_encode_rfc4648_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(base32_encode_rfc4648_reference(val, aux), base32_encode_rfc4648(val, aux));
        }
    }
}
