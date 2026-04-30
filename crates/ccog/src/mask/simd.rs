//! `portable_simd`-backed AND/OR for [`super::ktier::K256`].
//!
//! Nightly feature `portable_simd` enables a single `u64x4` lane covering a
//! K256 mask. `forbid(unsafe_code)` is preserved — the portable SIMD API is
//! safe.
//!
//! Stable fallback: scalar word-by-word loops in the trait impls in
//! [`super::ktier`]. This module is only compiled with `feature = "nightly"`.

use super::ktier::{K128, K256};
use core::simd::u64x4;

/// SIMD AND of two 256-bit masks. Equivalent to `a & b` from the trait.
#[inline]
pub fn k256_and(a: K256, b: K256) -> K256 {
    let av = u64x4::from_array(a.0);
    let bv = u64x4::from_array(b.0);
    K256((av & bv).to_array())
}

/// SIMD OR of two 256-bit masks. Equivalent to `a | b` from the trait.
#[inline]
pub fn k256_or(a: K256, b: K256) -> K256 {
    let av = u64x4::from_array(a.0);
    let bv = u64x4::from_array(b.0);
    K256((av | bv).to_array())
}

/// SIMD AND of two 128-bit masks (zero-extended to four lanes).
#[inline]
pub fn k128_and(a: K128, b: K128) -> K128 {
    let av = u64x4::from_array([a.0[0], a.0[1], 0, 0]);
    let bv = u64x4::from_array([b.0[0], b.0[1], 0, 0]);
    let r = (av & bv).to_array();
    K128([r[0], r[1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::ktier::KMask;

    #[test]
    fn k256_simd_and_matches_scalar() {
        let a = K256::bit(3) | K256::bit(130);
        let b = K256::bit(3) | K256::bit(200);
        assert_eq!(k256_and(a, b), a & b);
    }

    #[test]
    fn k256_simd_or_matches_scalar() {
        let a = K256::bit(3);
        let b = K256::bit(200);
        assert_eq!(k256_or(a, b), a | b);
    }
}
