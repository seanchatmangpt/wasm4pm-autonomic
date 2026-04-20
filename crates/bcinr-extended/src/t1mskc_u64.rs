//! Branchless Implementation: t1mskc_u64
#[inline(always)]
#[no_mangle]
pub fn t1mskc_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (!val) | val.wrapping_add(1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn t1mskc_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(t1mskc_u64_reference(val, aux), t1mskc_u64(val, aux));
        }
    }
}
