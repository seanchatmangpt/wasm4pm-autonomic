//! Branchless Implementation: count_min_sketch_query
#[inline(always)]
#[no_mangle]
pub fn count_min_sketch_query(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn count_min_sketch_query_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(count_min_sketch_query_reference(val, aux), count_min_sketch_query(val, aux));
        }
    }
}
