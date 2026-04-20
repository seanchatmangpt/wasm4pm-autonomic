//! Branchless Implementation: bset_u64
#[inline(always)]
#[no_mangle]
pub fn bset_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    val | (1u64.wrapping_shl((aux & 63) as u32))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bset_u64_reference(val: u64, aux: u64) -> u64 {
        val | (1 << (aux % 64))
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bset_u64_reference(val, aux), bset_u64(val, aux));
        }
    }
}
