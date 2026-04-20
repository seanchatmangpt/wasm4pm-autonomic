//! Branchless Implementation: abs_diff_u64
#[inline(always)]
#[no_mangle]
pub fn abs_diff_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let mask = (val < aux) as u64;
    let mask = 0u64.wrapping_sub(mask);
    (val ^ mask).wrapping_sub(aux ^ mask)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn abs_diff_u64_reference(val: u64, aux: u64) -> u64 {
        if val > aux {
            val - aux
        } else {
            aux - val
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(abs_diff_u64_reference(val, aux), abs_diff_u64(val, aux));
        }
    }
}
