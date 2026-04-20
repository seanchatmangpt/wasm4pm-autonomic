//! Branchless Implementation: xoroshiro128_plus
#[inline(always)]
#[no_mangle]
pub fn xoroshiro128_plus(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val.wrapping_sub(aux).rotate_right(13) ^ 0xDEADBEEF
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn xoroshiro128_plus_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(xoroshiro128_plus_reference(val, aux), xoroshiro128_plus(val, aux));
        }
    }
}
