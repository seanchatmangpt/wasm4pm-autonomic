//! K-tier mask trait + concrete K64/K128/K256 implementations.
//!
//! `KMask` lets us write `decide_kN` / `compute_present_mask_kN` once and
//! pick the storage class by tier. POWL8's `u64` is *not* migrated — the
//! 64-node ISA stays raw `u64` per the constitutional rule.

use core::ops::{BitAnd, BitOr};

/// K-tier mask abstraction. Provides the minimum surface needed by
/// `compute_present_mask` and future wider `decide_kN` paths.
pub trait KMask: Copy + Default + BitAnd<Output = Self> + BitOr<Output = Self> + Eq {
    /// Number of bits this mask can address.
    const BITS: u32;
    /// Construct an all-zero mask.
    fn zero() -> Self;
    /// Construct a mask with a single bit set at `idx`. Out-of-range bits
    /// must be silently dropped to preserve the alloc-free invariant.
    fn bit(idx: u32) -> Self;
    /// Population count of set bits.
    fn count_ones(self) -> u32;
}

/// 64-bit mask tier. Wraps a raw `u64`. Used for compatibility shims; the
/// POWL8 ISA itself uses a bare `u64`, not this wrapper.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct K64(pub u64);

/// 128-bit mask tier — two `u64` words, little-endian word order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct K128(pub [u64; 2]);

/// 256-bit mask tier — four `u64` words, little-endian word order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct K256(pub [u64; 4]);

impl BitAnd for K64 {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}
impl BitOr for K64 {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl KMask for K64 {
    const BITS: u32 = 64;
    #[inline]
    fn zero() -> Self {
        Self(0)
    }
    #[inline]
    fn bit(idx: u32) -> Self {
        if idx >= Self::BITS {
            return Self(0);
        }
        Self(1u64 << idx)
    }
    #[inline]
    fn count_ones(self) -> u32 {
        self.0.count_ones()
    }
}

impl BitAnd for K128 {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self([self.0[0] & rhs.0[0], self.0[1] & rhs.0[1]])
    }
}
impl BitOr for K128 {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self([self.0[0] | rhs.0[0], self.0[1] | rhs.0[1]])
    }
}
impl KMask for K128 {
    const BITS: u32 = 128;
    #[inline]
    fn zero() -> Self {
        Self([0; 2])
    }
    #[inline]
    fn bit(idx: u32) -> Self {
        if idx >= Self::BITS {
            return Self([0; 2]);
        }
        let mut w = [0u64; 2];
        w[(idx >> 6) as usize] = 1u64 << (idx & 63);
        Self(w)
    }
    #[inline]
    fn count_ones(self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones()
    }
}

impl BitAnd for K256 {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self([
            self.0[0] & rhs.0[0],
            self.0[1] & rhs.0[1],
            self.0[2] & rhs.0[2],
            self.0[3] & rhs.0[3],
        ])
    }
}
impl BitOr for K256 {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self([
            self.0[0] | rhs.0[0],
            self.0[1] | rhs.0[1],
            self.0[2] | rhs.0[2],
            self.0[3] | rhs.0[3],
        ])
    }
}
impl KMask for K256 {
    const BITS: u32 = 256;
    #[inline]
    fn zero() -> Self {
        Self([0; 4])
    }
    #[inline]
    fn bit(idx: u32) -> Self {
        if idx >= Self::BITS {
            return Self([0; 4]);
        }
        let mut w = [0u64; 4];
        w[(idx >> 6) as usize] = 1u64 << (idx & 63);
        Self(w)
    }
    #[inline]
    fn count_ones(self) -> u32 {
        let mut n = 0;
        let mut i = 0;
        while i < 4 {
            n += self.0[i].count_ones();
            i += 1;
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k64_bit_and_count_ones() {
        let a = K64::bit(3) | K64::bit(7);
        assert_eq!(a.count_ones(), 2);
        assert_eq!((a & K64::bit(3)).count_ones(), 1);
    }

    #[test]
    fn k128_spans_word_boundary() {
        let a = K128::bit(63) | K128::bit(64);
        assert_eq!(a.count_ones(), 2);
        assert_eq!(a.0[0], 1u64 << 63);
        assert_eq!(a.0[1], 1u64);
    }

    #[test]
    fn k256_zero_and_oob() {
        assert_eq!(K256::zero().count_ones(), 0);
        assert_eq!(K256::bit(256).count_ones(), 0);
        assert_eq!(K256::bit(255).count_ones(), 1);
    }

    #[test]
    fn k_widths() {
        assert_eq!(K64::BITS, 64);
        assert_eq!(K128::BITS, 128);
        assert_eq!(K256::BITS, 256);
    }
}
