//! Branchless Implementation: is_contiguous_mask_u64
#[inline(always)]
#[no_mangle]
pub fn is_contiguous_mask_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let b = val & val.wrapping_neg();
    let t = val.wrapping_add(b);
    (t & val == 0 && val != 0) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn is_contiguous_mask_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(is_contiguous_mask_u64_reference(val, aux), is_contiguous_mask_u64(val, aux));
        }
    }
}
