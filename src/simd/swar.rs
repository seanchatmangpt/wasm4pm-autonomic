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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_fire_branchless_enabled() {
        let marking = SwarMarking64::new(0b1111);
        let req = &[0b0011];
        let out = &[0b1100];
        let (new_marking, fired) = marking.try_fire_branchless(req, out);
        assert!(fired);
        assert_eq!(new_marking.words[0], 0b1100);
    }

    #[test]
    fn test_try_fire_branchless_disabled() {
        let marking = SwarMarking64::new(0b0001);
        let req = &[0b1110]; // Requires bits not present
        let out = &[0b0000];
        let (_new_marking, fired) = marking.try_fire_branchless(req, out);
        assert!(!fired);
    }

    #[test]
    fn test_swar_marking_new_sets_word_zero() {
        let marking = SwarMarking64::new(0xDEADBEEF);
        assert_eq!(marking.words[0], 0xDEADBEEF);
    }
}
