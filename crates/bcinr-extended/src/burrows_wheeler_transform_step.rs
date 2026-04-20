//! Branchless Implementation: burrows_wheeler_transform_step
#[inline(always)]
#[no_mangle]
pub fn burrows_wheeler_transform_step(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    (val ^ aux).count_ones() as u64 | (val.rotate_left(11))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn burrows_wheeler_transform_step_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(burrows_wheeler_transform_step_reference(val, aux), burrows_wheeler_transform_step(val, aux));
        }
    }
}
