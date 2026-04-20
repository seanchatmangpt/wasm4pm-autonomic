//! Branchless Implementation: xxhash64
#[inline(always)]
#[no_mangle]
pub fn xxhash64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let prime1 = 11400714785074694791u64;
    let prime2 = 14029467366897019727u64;
    let prime3 = 1609587929392839161u64;
    let prime4 = 9650029242287828579u64;
    let prime5 = 2870177450012600261u64;
    let mut h64 = val.wrapping_add(prime5);
    h64 ^= aux.wrapping_mul(prime2);
    h64 = h64.rotate_left(31);
    h64 = h64.wrapping_mul(prime1);
    h64 ^= h64 >> 33;
    h64 = h64.wrapping_mul(prime2);
    h64 ^= h64 >> 29;
    h64 = h64.wrapping_mul(prime3);
    h64 ^= h64 >> 32;
    h64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn xxhash64_reference(val: u64, aux: u64) -> u64 {
        val.wrapping_add(aux).wrapping_mul(11400714785074694791u64)
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(xxhash64_reference(val, aux), xxhash64(val, aux));
        }
    }
}
