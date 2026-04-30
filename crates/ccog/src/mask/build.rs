//! Const-fn mask builders.
//!
//! Out-of-range bit indices silently drop to zero so callers that compose
//! masks at compile time cannot accidentally panic the kernel.

use super::ktier::{K128, K256, K64};

/// Build a [`K64`] mask from an array of bit indices. Indices ≥ 64 are dropped.
#[inline]
pub const fn k64_from_bits<const N: usize>(bits: [u32; N]) -> K64 {
    let mut m: u64 = 0;
    let mut i = 0;
    while i < N {
        let b = bits[i];
        if b < 64 {
            m |= 1u64 << b;
        }
        i += 1;
    }
    K64(m)
}

/// Build a [`K128`] mask from an array of bit indices.
#[inline]
pub const fn k128_from_bits<const N: usize>(bits: [u32; N]) -> K128 {
    let mut w = [0u64; 2];
    let mut i = 0;
    while i < N {
        let b = bits[i];
        if b < 128 {
            w[(b >> 6) as usize] |= 1u64 << (b & 63);
        }
        i += 1;
    }
    K128(w)
}

/// Build a [`K256`] mask from an array of bit indices.
#[inline]
pub const fn k256_from_bits<const N: usize>(bits: [u32; N]) -> K256 {
    let mut w = [0u64; 4];
    let mut i = 0;
    while i < N {
        let b = bits[i];
        if b < 256 {
            w[(b >> 6) as usize] |= 1u64 << (b & 63);
        }
        i += 1;
    }
    K256(w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mask::ktier::KMask;

    #[test]
    fn k64_const_builder() {
        const M: K64 = k64_from_bits([1, 5, 63]);
        assert_eq!(M.count_ones(), 3);
    }

    #[test]
    fn k128_const_builder_spans_words() {
        const M: K128 = k128_from_bits([0, 64, 127]);
        assert_eq!(M.count_ones(), 3);
        assert_eq!(M.0[0], 1u64);
        assert_eq!(M.0[1], 1u64 | (1u64 << 63));
    }

    #[test]
    fn k256_const_builder_drops_oob() {
        const M: K256 = k256_from_bits([0, 255, 999]);
        assert_eq!(M.count_ones(), 2);
    }
}
