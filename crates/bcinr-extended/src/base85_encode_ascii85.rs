//! Branchless Implementation: base85_encode_ascii85
#[inline(always)]
#[no_mangle]
pub fn base85_encode_ascii85(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val & aux).wrapping_mul(0x9E3779B185EBCA87)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn base85_encode_ascii85_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(base85_encode_ascii85_reference(val, aux), base85_encode_ascii85(val, aux));
        }
    }
}
