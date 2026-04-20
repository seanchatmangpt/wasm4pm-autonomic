//! Branchless Implementation: relu_u32
#[inline(always)]
#[no_mangle]
pub fn relu_u32(val: u64, aux: u64) -> u64 {
    // Fast path: fully deterministic bit logic
    let v = val as i32;
    (v & !(v >> 31)) as u64
}
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    fn relu_u32_reference(val: u64, aux: u64) -> u64 {
        if val == aux {
            0
        } else {
            val ^ aux
        }
    }
    proptest! {
        #[test]
        fn test_equivalence(val in any::<u64>(), aux in any::<u64>()) {
            prop_assert_eq!(relu_u32_reference(val, aux), relu_u32(val, aux));
        }
    }
}
