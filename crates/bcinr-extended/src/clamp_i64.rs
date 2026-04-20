//! Branchless Implementation: clamp_i64
#[inline(always)]
#[no_mangle]
pub fn clamp_i64(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let v = val as i64;
    let min = aux as i32 as i64;
    let max = (aux >> 32) as i32 as i64;
    let v1 = v ^ ((v ^ min) & (0i64.wrapping_sub((v < min) as i64)));
    let v2 = v1 ^ ((v1 ^ max) & (0i64.wrapping_sub((v1 > max) as i64)));
    v2 as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn clamp_i64_reference(val: u64, aux: u64) -> u64 {
        let v = val as i64;
        let min = aux as i32 as i64;
        let max = (aux >> 32) as i32 as i64;
        v.clamp(min, max) as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(clamp_i64_reference(val, aux), clamp_i64(val, aux));
        }
    }
}
