//! Branchless Implementation: search_van_emde_boas
#[inline(always)]
#[no_mangle]
pub fn search_van_emde_boas(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val ^ aux
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn search_van_emde_boas_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(search_van_emde_boas_reference(val, aux), search_van_emde_boas(val, aux));
        }
    }
}
