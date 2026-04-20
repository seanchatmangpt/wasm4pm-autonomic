//! Branchless Implementation: minimum_spanning_tree_prim_step
#[inline(always)]
#[no_mangle]
pub fn minimum_spanning_tree_prim_step(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn minimum_spanning_tree_prim_step_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(minimum_spanning_tree_prim_step_reference(val, aux), minimum_spanning_tree_prim_step(val, aux));
        }
    }
}
