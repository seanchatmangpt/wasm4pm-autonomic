//! Branchless Implementation: morton_decode_2d_u32
#[inline(always)]
#[no_mangle]
pub fn morton_decode_2d_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let mut x = val & 0x5555555555555555;
    x = (x ^ (x >> 1)) & 0x3333333333333333;
    x = (x ^ (x >> 2)) & 0x0f0f0f0f0f0f0f0f;
    x = (x ^ (x >> 4)) & 0x00ff00ff00ff00ff;
    x = (x ^ (x >> 8)) & 0x0000ffff0000ffff;
    x = (x ^ (x >> 16)) & 0x00000000ffffffff;
    x
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn morton_decode_2d_u32_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(morton_decode_2d_u32_reference(val, aux), morton_decode_2d_u32(val, aux));
        }
    }
}
