//! Branchless Implementation: next_lexicographic_permutation_u64
#[inline(always)]
#[no_mangle]
pub fn next_lexicographic_permutation_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let t = val | val.wrapping_sub(1);
    let c = !t & t.wrapping_add(1);
    let tz = val.trailing_zeros();
    let shift = tz.wrapping_add(1) & 0x3F;
    let o = (c.wrapping_sub(1)).wrapping_shr(shift);
    (t.wrapping_add(1) | o) * (val != 0) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn next_lexicographic_permutation_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(next_lexicographic_permutation_u64_reference(val, aux), next_lexicographic_permutation_u64(val, aux));
        }
    }
}
