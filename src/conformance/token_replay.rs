//! Zero-allocation, branchless token-based replay engine.
//!
//! This module implements a highly optimized token-based replay engine
//! that compares an execution trace against a POWL process model (up to 64 places).
//! It serves as the absolute empirical authority for Compiled Cognition.

/// Represents the token requirements and outputs of a transition.
#[derive(Debug, Clone, Copy)]
pub struct TransitionMask {
    pub consume: u64,
    pub produce: u64,
}

/// Tracks the evolving state during token replay.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReplayState {
    pub marking: u64,
    pub missing_tokens: u32,
    pub consumed_tokens: u32,
    pub produced_tokens: u32,
}

impl ReplayState {
    /// Fire a transition using branchless bitwise operations.
    #[inline(always)]
    pub fn fire(&mut self, mask: TransitionMask) {
        // Calculate missing tokens (required by consume but not in current marking)
        let missing = mask.consume & !self.marking;
        
        // Branchless updates of running totals
        self.missing_tokens += missing.count_ones();
        self.consumed_tokens += mask.consume.count_ones();
        self.produced_tokens += mask.produce.count_ones();

        // Branchless state evolution:
        // Remove consumed tokens and add produced tokens.
        // We do not need to explicitly "add" the missing tokens before consuming
        // because `& !mask.consume` implicitly handles both present and missing.
        self.marking = (self.marking & !mask.consume) | mask.produce;
    }
}

/// A highly optimized engine for checking conformance of traces against a POWL model.
pub struct TokenReplayEngine {
    // 256 max transitions allows zero-cost branchless indexing using a u8 trace event.
    transitions: [TransitionMask; 256],
    initial_marking: u64,
    final_marking: u64,
}

impl TokenReplayEngine {
    /// Create a new replay engine from transition masks and markings.
    pub fn new(transitions: &[TransitionMask], initial_marking: u64, final_marking: u64) -> Self {
        let mut engine_transitions = [TransitionMask { consume: 0, produce: 0 }; 256];
        let copy_len = transitions.len().min(256);
        engine_transitions[..copy_len].copy_from_slice(&transitions[..copy_len]);
        
        Self {
            transitions: engine_transitions,
            initial_marking,
            final_marking,
        }
    }

    /// Replay an execution trace without any allocations or conditional branches.
    /// `trace` is a slice of u8, where each value maps to a transition index.
    #[inline(always)]
    pub fn replay_trace(&self, trace: &[u8]) -> ReplayState {
        let mut state = ReplayState {
            marking: self.initial_marking,
            missing_tokens: 0,
            consumed_tokens: 0,
            produced_tokens: self.initial_marking.count_ones(),
        };

        // Core execution loop: zero allocation, zero branching
        for &event_idx in trace {
            // Because event_idx is u8 and transitions array is size 256,
            // this indexing can be perfectly optimized by the compiler without bounds checks.
            let mask = self.transitions[event_idx as usize];
            state.fire(mask);
        }

        // Handle final marking exactly like a dummy transition consumption
        let missing_final = self.final_marking & !state.marking;
        state.missing_tokens += missing_final.count_ones();
        state.consumed_tokens += self.final_marking.count_ones();
        state.marking &= !self.final_marking;

        state
    }
    
    /// Calculate standard fitness score from the resulting replay state.
    pub fn calculate_fitness(state: &ReplayState) -> f64 {
        let remaining_tokens = state.marking.count_ones();
        let total_tokens_needed = state.consumed_tokens + state.missing_tokens;
        
        if total_tokens_needed == 0 {
            1.0
        } else {
            1.0 - (state.missing_tokens as f64 + remaining_tokens as f64)
                / (total_tokens_needed as f64 + state.produced_tokens as f64)
        }
    }
}
