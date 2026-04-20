//! Branchless Implementation: url_encode_branchless
#[inline(always)]
#[no_mangle]
pub fn url_encode_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_add(aux) ^ (val.rotate_left(7))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn url_encode_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(url_encode_branchless_reference(val, aux), url_encode_branchless(val, aux));
        }
    }
}
