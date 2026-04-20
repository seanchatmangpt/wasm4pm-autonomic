//! Branchless Implementation: manhattan_dist_u32x2
#[inline(always)]
#[no_mangle]
pub fn manhattan_dist_u32x2(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let x1 = (val & 0xFFFFFFFF) as i64;
    let y1 = (val >> 32) as i64;
    let x2 = (aux & 0xFFFFFFFF) as i64;
    let y2 = (aux >> 32) as i64;
    x1.abs_diff(x2) + y1.abs_diff(y2)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn manhattan_dist_u32x2_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(manhattan_dist_u32x2_reference(val, aux), manhattan_dist_u32x2(val, aux));
        }
    }
}
