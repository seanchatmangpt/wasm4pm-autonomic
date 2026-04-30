//! Posture state machine: transitions Calm → Alert → Engaged → Settled by signal count.

use crate::verdict::PackPosture;

/// Posture state machine driven by per-tick confirmed-signal counts.
#[derive(Clone, Debug)]
pub struct PostureMachine {
    current: PackPosture,
}

impl PostureMachine {
    /// Create a new machine starting at `Calm`.
    pub fn new() -> Self { Self { current: PackPosture::Calm } }

    /// Current posture (no mutation).
    pub fn current(&self) -> PackPosture { self.current }

    /// Update posture based on this tick's `signal_count` and return the new state.
    /// 0 → Calm, 1 → Alert, 2-3 → Engaged, 4+ → Settled.
    pub fn observe(&mut self, signal_count: usize) -> PackPosture {
        self.current = match signal_count {
            0 => PackPosture::Calm,
            1 => PackPosture::Alert,
            2 | 3 => PackPosture::Engaged,
            _ => PackPosture::Settled,
        };
        self.current
    }

    /// Manually reset posture to `Calm`.
    pub fn settle(&mut self) { self.current = PackPosture::Calm; }
}

impl Default for PostureMachine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_observes_calm() {
        let mut m = PostureMachine::new();
        assert_eq!(m.observe(0), PackPosture::Calm);
    }
    #[test]
    fn transitions_through_bands() {
        let mut m = PostureMachine::new();
        assert_eq!(m.observe(1), PackPosture::Alert);
        assert_eq!(m.observe(2), PackPosture::Engaged);
        assert_eq!(m.observe(3), PackPosture::Engaged);
        assert_eq!(m.observe(7), PackPosture::Settled);
    }
    #[test]
    fn settle_resets() {
        let mut m = PostureMachine::new();
        m.observe(5);
        m.settle();
        assert_eq!(m.current(), PackPosture::Calm);
    }
    #[test]
    fn display_strings() {
        assert_eq!(format!("{}", PackPosture::Calm), "calm");
        assert_eq!(format!("{}", PackPosture::Settled), "settled");
    }
}
