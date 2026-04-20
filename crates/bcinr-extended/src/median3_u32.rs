//! Branchless Implementation: median3_u32
#[inline(always)]
#[no_mangle]
pub fn median3_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn median3_u32_reference(val: u64, aux: u64) -> u64 {
        let a = val as u32;
        let b = (val >> 32) as u32;
        let c = aux as u32;
        let mut arr = [a, b, c];
        arr.sort();
        arr[1] as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(median3_u32_reference(val, aux), median3_u32(val, aux));
        }
    }
}
