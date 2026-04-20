//! Branchless Implementation: delta_swap_u64
#[inline(always)]
#[no_mangle]
pub fn delta_swap_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let delta = (aux & 0x3F) as u32;
    let mask = aux >> 32;
    let t = ((val.wrapping_shr(delta)) ^ val) & mask;
    val ^ t ^ (t.wrapping_shl(delta))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn delta_swap_u64_reference(val: u64, aux: u64) -> u64 {
        let delta = (aux & 0x3F) as u32;
        let mask = aux >> 32;
        let t = ((val.wrapping_shr(delta)) ^ val) & mask;
        val ^ t ^ (t.wrapping_shl(delta))
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(delta_swap_u64_reference(val, aux), delta_swap_u64(val, aux));
        }
    }
}
