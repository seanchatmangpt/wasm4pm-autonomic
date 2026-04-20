//! Branchless Implementation: move_to_front_branchless
#[inline(always)]
#[no_mangle]
pub fn move_to_front_branchless(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_add(aux) ^ (val.rotate_left(7))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn move_to_front_branchless_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(move_to_front_branchless_reference(val, aux), move_to_front_branchless(val, aux));
        }
    }
}
