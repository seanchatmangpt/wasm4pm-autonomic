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
        let mut is_enabled = true;
        for (i, &r) in req.iter().enumerate().take(WORDS) {
            if (self.words[i] & r) != r {
                is_enabled = false;
            }
        }

        let mut next_words = [0u64; WORDS];
        let cond = is_enabled as u64;

        for (i, &next_val) in out.iter().enumerate().take(WORDS) {
            let next = (self.words[i] & !req[i]) | next_val;
            next_words[i] = select_u64(cond, next, self.words[i]);
        }

        (Self { words: next_words }, is_enabled)
    }
}

pub type SwarMarking64 = SwarMarking<1>;
