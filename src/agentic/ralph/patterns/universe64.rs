//! Pattern: Universe64
//!
//! Purpose: Represents a fixed 32 KiB deterministic Boolean universe state ($U_t \in \mathbb{B}^{64^3}$).
//! Primitive dependencies: Array indexing, bitwise operations (AND, OR, XOR, NOT), popcount.
//! Input contract: Transitions must specify valid input and output masks.
//! Output contract: `apply_transition` returns the new universe and a success mask without branching on admissibility.
//! Memory contract: Zero heap allocations. The universe is a fixed `[u64; 4096]` array.
//! Branch contract: State updates are computed branchlessly.
//! Capacity contract: Exactly 262,144 boolean facts.
//! Proof artifact: A rolling deterministic receipt of applied transitions.



/// Represents a 3D coordinate in the Universe64 lattice: (domain, cell, place).
/// All values must be in the range [0, 63].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UCoord {
    pub domain: u8,
    pub cell: u8,
    pub place: u8,
}

impl UCoord {
    /// Creates a new UCoord, panicking in debug mode if coordinates are out of bounds.
    #[inline(always)]
    pub const fn new(domain: u8, cell: u8, place: u8) -> Self {
        debug_assert!(domain < 64 && cell < 64 && place < 64, "UCoord out of bounds");
        Self { domain, cell, place }
    }

    /// Converts the 3D coordinate into a flat word index [0, 4095] and a bit offset [0, 63].
    #[inline(always)]
    pub const fn to_index(&self) -> (usize, u32) {
        let word_index = (self.domain as usize * 64) + (self.cell as usize);
        (word_index, self.place as u32)
    }
}

/// A non-cryptographic, deterministic rolling receipt for substrate telemetry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UReceipt {
    pub current_hash: u64,
    pub steps: u64,
}

impl UReceipt {
    pub const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    pub const FNV_PRIME: u64 = 0x100000001b3;

    pub const fn new() -> Self {
        Self {
            current_hash: Self::FNV_OFFSET,
            steps: 0,
        }
    }

    #[inline(always)]
    const fn mix(mut h: u64, x: u64) -> u64 {
        h ^= x;
        h = h.wrapping_mul(Self::FNV_PRIME);
        h
    }

    /// Mixes transition details into the receipt.
    #[inline(always)]
    pub fn record_transition(&mut self, word_idx: usize, input_mask: u64, output_mask: u64, fired_mask: u64) {
        let mut h = self.current_hash;
        h = Self::mix(h, self.steps);
        h = Self::mix(h, word_idx as u64);
        h = Self::mix(h, input_mask);
        h = Self::mix(h, output_mask);
        h = Self::mix(h, fired_mask);
        self.current_hash = h;
        self.steps = self.steps.wrapping_add(1);
    }
}

/// The universal deterministic state object.
/// 262,144 boolean facts packed into 4096 64-bit words (32 KiB).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(C, align(64))] // Align to typical cache line size
pub struct Universe64 {
    pub data: [u64; 4096],
}

impl Universe64 {
    /// Creates an empty (all zeros) universe.
    pub const fn empty() -> Self {
        Self { data: [0; 4096] }
    }

    /// Sets the fact at the given coordinate to true.
    #[inline(always)]
    pub fn set(&mut self, coord: UCoord) {
        let (idx, bit) = coord.to_index();
        self.data[idx] |= 1u64 << bit;
    }

    /// Sets the fact at the given coordinate to false.
    #[inline(always)]
    pub fn clear(&mut self, coord: UCoord) {
        let (idx, bit) = coord.to_index();
        self.data[idx] &= !(1u64 << bit);
    }

    /// Returns true if the fact at the given coordinate is true.
    #[inline(always)]
    pub fn get(&self, coord: UCoord) -> bool {
        let (idx, bit) = coord.to_index();
        (self.data[idx] & (1u64 << bit)) != 0
    }

    /// Computes the exact Hamming distance (number of differing facts) between two universes.
    /// This is typically a T2 orchestration operation due to scanning 32 KiB.
    #[inline(always)]
    pub fn conformance_distance(&self, expected: &Self) -> usize {
        let mut dist = 0;
        // Using chunks to encourage auto-vectorization
        for (a, b) in self.data.iter().zip(expected.data.iter()) {
            dist += (a ^ b).count_ones() as usize;
        }
        dist
    }

    /// Applies a local transition to a specific word (cell) branchlessly.
    /// Returns the firing mask (!0 if fired, 0 if not).
    /// This is a T1-admissible operation.
    #[inline(always)]
    pub fn apply_local_transition(&mut self, word_idx: usize, input_mask: u64, output_mask: u64) -> u64 {
        let current = self.data[word_idx];
        
        // enabled = (current & input_mask) == input_mask
        // We want a full u64 mask (!0 for true, 0 for false).
        // If current & input_mask == input_mask, then (current & input_mask) ^ input_mask == 0.
        let diff = (current & input_mask) ^ input_mask;
        // If diff == 0, enabled_mask should be !0. If diff != 0, enabled_mask should be 0.
        // We can do this branchlessly:
        let is_zero = ((diff | diff.wrapping_neg()) >> 63) ^ 1;
        let enabled_mask = 0u64.wrapping_sub(is_zero);

        let candidate = (current & !input_mask) | output_mask;
        
        // Select candidate if enabled, else keep current
        self.data[word_idx] = (candidate & enabled_mask) | (current & !enabled_mask);
        
        enabled_mask
    }
    
    /// Applies a boundary transition between two distinct cells.
    /// Evaluates if the combined input conditions in both cells are met, 
    /// and if so, applies the outputs to both cells branchlessly.
    /// This is an inter-cell T1/T2 boundary operation.
    #[inline(always)]
    pub fn apply_boundary_transition(
        &mut self, 
        idx_a: usize, 
        in_a: u64, 
        out_a: u64,
        idx_b: usize, 
        in_b: u64, 
        out_b: u64
    ) -> u64 {
        debug_assert!(idx_a != idx_b, "Boundary transitions require distinct cells");
        let val_a = self.data[idx_a];
        let val_b = self.data[idx_b];
        
        let diff_a = (val_a & in_a) ^ in_a;
        let diff_b = (val_b & in_b) ^ in_b;
        let total_diff = diff_a | diff_b;
        
        let is_zero = ((total_diff | total_diff.wrapping_neg()) >> 63) ^ 1;
        let enabled_mask = 0u64.wrapping_sub(is_zero);
        
        let cand_a = (val_a & !in_a) | out_a;
        let cand_b = (val_b & !in_b) | out_b;
        
        self.data[idx_a] = (cand_a & enabled_mask) | (val_a & !enabled_mask);
        self.data[idx_b] = (cand_b & enabled_mask) | (val_b & !enabled_mask);
        
        enabled_mask
    }

    /// Applies transitions to a sparse set of active cells branchlessly.
    /// `transitions` is a slice of (word_index, input_mask, output_mask).
    /// Returns the number of transitions that successfully fired.
    /// This is a T1 candidate operation if the number of transitions is small.
    #[inline(always)]
    pub fn apply_sparse_transitions(&mut self, transitions: &[(usize, u64, u64)], receipt: &mut UReceipt) -> usize {
        let mut fired_count = 0;
        for &(idx, i_mask, o_mask) in transitions {
            let fired_mask = self.apply_local_transition(idx, i_mask, o_mask);
            receipt.record_transition(idx, i_mask, o_mask, fired_mask);
            fired_count += (fired_mask & 1) as usize;
        }
        fired_count
    }
}

