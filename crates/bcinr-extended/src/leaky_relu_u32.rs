//! Branchless Implementation: leaky_relu_u32
#[inline(always)]
#[no_mangle]
pub fn leaky_relu_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let v = val as i32;
    let m = v >> 31;
    (v & !m) as u64 | ((v / 10) as i64 & m as i64) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn leaky_relu_u32_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(leaky_relu_u32_reference(val, aux), leaky_relu_u32(val, aux));
        }
    }
}
