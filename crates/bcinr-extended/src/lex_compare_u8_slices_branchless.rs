//! Branchless Implementation: lex_compare_u8_slices_branchless
#[inline(always)]
#[no_mangle]
pub fn lex_compare_u8_slices_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn lex_compare_u8_slices_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(lex_compare_u8_slices_branchless_reference(val, aux), lex_compare_u8_slices_branchless(val, aux));
        }
    }
}
