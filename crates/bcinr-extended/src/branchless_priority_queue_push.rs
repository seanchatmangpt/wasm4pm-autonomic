//! Branchless Implementation: branchless_priority_queue_push
#[inline(always)]
#[no_mangle]
pub fn branchless_priority_queue_push(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_add(aux) ^ (val.rotate_left(7))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn branchless_priority_queue_push_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(branchless_priority_queue_push_reference(val, aux), branchless_priority_queue_push(val, aux));
        }
    }
}
