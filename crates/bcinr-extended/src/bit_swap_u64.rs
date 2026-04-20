//! Branchless Implementation: bit_swap_u64
#[inline(always)]
#[no_mangle]
pub fn bit_swap_u64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let i = (aux & 0x3F) as u32;
    let j = ((aux >> 8) & 0x3F) as u32;
    let bit_i = (val.wrapping_shr(i)) & 1;
    let bit_j = (val.wrapping_shr(j)) & 1;
    let xor_val = bit_i ^ bit_j;
    val ^ ((xor_val.wrapping_shl(i)) | (xor_val.wrapping_shl(j)))
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn bit_swap_u64_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(bit_swap_u64_reference(val, aux), bit_swap_u64(val, aux));
        }
    }
}
