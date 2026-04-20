//! Branchless Implementation: point_in_polygon_branchless
#[inline(always)]
#[no_mangle]
pub fn point_in_polygon_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn point_in_polygon_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(point_in_polygon_branchless_reference(val, aux), point_in_polygon_branchless(val, aux));
        }
    }
}
