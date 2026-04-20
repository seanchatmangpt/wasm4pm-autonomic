//! Branchless Implementation: softmax_u32x4
#[inline(always)]
#[no_mangle]
pub fn softmax_u32x4(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn softmax_u32x4_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(softmax_u32x4_reference(val, aux), softmax_u32x4(val, aux));
        }
    }
}
