//! Branchless Implementation: space_saving_add
#[inline(always)]
#[no_mangle]
pub fn space_saving_add(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn space_saving_add_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(space_saving_add_reference(val, aux), space_saving_add(val, aux));
        }
    }
}
