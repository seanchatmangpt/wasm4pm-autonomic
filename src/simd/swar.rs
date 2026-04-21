//! Hyper-optimized branchless SWAR (SIMD Within A Register) token replay kernel.
//! Exploits 64-bit word parallelism to replay multiple tokens simultaneously without data-dependent branching.

use crate::utils::bitset::select_u64;

#[derive(Clone, Copy)]
pub struct SwarMarking<const WORDS: usize> {
    pub words: [u64; WORDS],
}

impl<const WORDS: usize> SwarMarking<WORDS> {
    pub fn new(val: u64) -> Self {
        let mut words = [0u64; WORDS];
        if WORDS > 0 {
            words[0] = val;
        }
        Self { words }
    }

    /// Fire a transition using pure branchless mask calculus and BCINR-style select.
    /// Returns (new_marking, was_fired).
    #[inline(always)]
    pub fn try_fire_branchless(&self, req: &[u64; WORDS], out: &[u64; WORDS]) -> (Self, bool) {
        use crate::utils::dense_kernel::KBitSet;
        
        let current = KBitSet { words: self.words };
        let required = KBitSet { words: *req };
        
        let cond = current.is_enabled_mask(required);
        let is_enabled = cond != 0;

        let mut next_words = [0u64; WORDS];

        for i in 0..WORDS {
            let next = (self.words[i] & !req[i]) | out[i];
            next_words[i] = select_u64(cond, next, self.words[i]);
        }

        (Self { words: next_words }, is_enabled)
    }
}

pub type SwarMarking64 = SwarMarking<1>;
