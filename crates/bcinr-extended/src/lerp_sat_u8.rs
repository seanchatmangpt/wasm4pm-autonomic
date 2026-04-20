//! Branchless Implementation: lerp_sat_u8
#[inline(always)]
#[no_mangle]
pub fn lerp_sat_u8(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let s = (val & 0xFF) as i32;
    let e = ((val >> 8) & 0xFF) as i32;
    let t = (aux & 0xFF) as i32;
    (s + ((e - s) * t + 127) / 255) as u8 as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn lerp_sat_u8_reference(val: u64, aux: u64) -> u64 {
        let s = (val & 0xFF) as f32;
        let e = ((val >> 8) & 0xFF) as f32;
        let t = (aux & 0xFF) as f32 / 255.0;
        (s + (e - s) * t).round() as u64
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(lerp_sat_u8_reference(val, aux), lerp_sat_u8(val, aux));
        }
    }
}
