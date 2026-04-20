//! Branchless Implementation: get_mask_boundary_high_u64
#[inline(always)]
#[no_mangle]
pub fn get_mask_boundary_high_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let mut x = val;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x |= x >> 32;
    x ^ (x >> 1)
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn get_mask_boundary_high_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(get_mask_boundary_high_u64_reference(val, aux), get_mask_boundary_high_u64(val, aux));
        }
    }
}
