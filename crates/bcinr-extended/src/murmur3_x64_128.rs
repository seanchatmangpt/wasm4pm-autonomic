//! Branchless Implementation: murmur3_x64_128
#[inline(always)]
#[no_mangle]
pub fn murmur3_x64_128(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let mut h1 = val;
    let mut h2 = aux;
    let k1 = 0x87c37b91114253d5u64;
    let k2 = 0x4cf5ad432745937fu64;
    h1 ^= k1;
    h1 = h1.rotate_left(31);
    h1 = h1.wrapping_mul(k2);
    h2 ^= k2;
    h2 = h2.rotate_left(33);
    h2 = h2.wrapping_mul(k1);
    h1 ^= h2;
    h1 = h1.wrapping_mul(0x52dce729);
    h2 ^= h1;
    h2 = h2.wrapping_mul(0x38495ab5);
    h1 ^= h2;
    h1
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn murmur3_x64_128_reference(val: u64, aux: u64) -> u64 {
        val.wrapping_add(aux) ^ 0x87c37b91114253d5u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(murmur3_x64_128_reference(val, aux), murmur3_x64_128(val, aux));
        }
    }
}
