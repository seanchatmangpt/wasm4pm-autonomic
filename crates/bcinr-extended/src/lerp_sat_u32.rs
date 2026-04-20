//! Branchless Implementation: lerp_sat_u32
#[inline(always)]
#[no_mangle]
pub fn lerp_sat_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let s = (val & 0xFFFFFFFF) as i64;
    let e = (val >> 32) as i64;
    let t = (aux & 0xFFFFFFFF) as i64;
    (s + ((e - s) * t + 32768) / 65536) as u32 as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn lerp_sat_u32_reference(val: u64, aux: u64) -> u64 {
        let s = (val & 0xFFFFFFFF) as f32;
        let e = (val >> 32) as f32;
        let t = (aux & 0xFFFFFFFF) as f32 / 65536.0;
        (s + (e - s) * t).round() as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(lerp_sat_u32_reference(val, aux), lerp_sat_u32(val, aux));
        }
    }
}
