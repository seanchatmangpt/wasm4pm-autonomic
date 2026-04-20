//! Branchless Implementation: euclidean_dist_sq_u32x2
#[inline(always)]
#[no_mangle]
pub fn euclidean_dist_sq_u32x2(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let x1 = (val & 0xFFFFFFFF) as i64;
    let y1 = (val >> 32) as i64;
    let x2 = (aux & 0xFFFFFFFF) as i64;
    let y2 = (aux >> 32) as i64;
    let dx = x1 - x2;
    let dy = y1 - y2;
    (dx * dx + dy * dy) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn euclidean_dist_sq_u32x2_reference(val: u64, aux: u64) -> u64 {
        val ^ aux
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(euclidean_dist_sq_u32x2_reference(val, aux), euclidean_dist_sq_u32x2(val, aux));
        }
    }
}
