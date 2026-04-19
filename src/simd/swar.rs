//! Hyper-optimized branchless SWAR (SIMD Within A Register) token replay kernel.
//! Exploits 64-bit word parallelism to replay multiple tokens simultaneously without data-dependent branching.

use crate::utils::bitset::select_u64;

#[derive(Clone, Copy)]
pub struct SwarMarking(pub u64);

impl SwarMarking {
    /// Fire a transition using pure branchless mask calculus and BCINR-style select.
    /// `req` = Required input marking mask.
    /// `out` = Output marking mask.
    /// Returns a tuple of (New Marking, Was Fired Successfully).
    #[inline(always)]
    pub fn try_fire_branchless(self, req: u64, out: u64) -> (Self, bool) {
        let m = self.0;
        
        // Identity: subset check (M & Req) == Req
        let is_enabled = (m & req) == req;
        
        // Transition calculus: next = (M & ~Req) | Out
        let next = (m & !req) | out;
        
        // Use BCINR-style select primitive
        let result = select_u64(is_enabled as u64, next, m);
        
        (SwarMarking(result), is_enabled)
    }

    /// Parallel check if multiple transitions (up to 4, 16-bit packed) are enabled.
    /// Uses SWAR parallelism to evaluate 4 lanes simultaneously.
    #[inline(always)]
    pub fn check_enabled_packed(state: u64, packed_reqs: u64) -> u64 {
        // Broadcast the 16-bit state across 4 lanes (64 bits total)
        let s4 = state | (state << 16) | (state << 32) | (state << 48);
        
        // Lane-wise check: (s4 & reqs) ^ reqs
        let diff = (s4 & packed_reqs) ^ packed_reqs;
        
        // SWAR bitmask trick to identify lanes that are zero
        let zero_lanes = !diff & (diff.wrapping_sub(0x0001000100010001));
        
        // Extract high bit indicators for each 16-bit lane
        zero_lanes & 0x8000800080008000
    }
}
