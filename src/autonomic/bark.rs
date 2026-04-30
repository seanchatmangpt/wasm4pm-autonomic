//! BarkEvent — ephemeral activation signals from dog-pack security layer.
//!
//! Bark represents a security-relevant signal: Watch (passive monitoring), Guard (active defense),
//! Detection (anomaly found), Herding (corrective action), Probe (diagnostic test), or Recorder (evidence).
//! Each BarkEvent carries source breed, signal hash, timestamp, and tighten flag for posture escalation.
//!
//! BarkEvents are stack-local, ephemeral signals emitted during observe() and consumed by propose().
//! They do NOT persist — the caller extends AutonomicState or other aggregators with history if needed.

/// Kind of bark signal emitted by a security breed.
#[derive(Debug, Clone, Copy)]
pub enum BarkKind {
    /// Passive monitoring — non-blocking observation
    Watch,
    /// Active defense — retaliation or blocking
    Guard,
    /// Anomaly detection — conformance or drift
    Detection,
    /// Corrective action — healing or repair proposal
    Herding,
    /// Diagnostic test — is_valid() probe
    Probe,
    /// Evidence recording — audit trail or proof
    Recorder,
}

/// Ephemeral security event emitted by a breed during observe() or propose().
///
/// BarkEvent carries:
/// - `kind`: Signal type (Watch, Guard, Detection, etc.)
/// - `source_breed`: Static name of originating breed (e.g., "guardian", "detector")
/// - `signal_hash`: FNV1a hash of the signal content (for idempotency)
/// - `timestamp_us`: Microsecond timestamp of emission
/// - `tighten`: True if this signal should upgrade pack posture
#[derive(Debug, Clone, Copy)]
pub struct BarkEvent {
    pub kind: BarkKind,
    pub source_breed: &'static str,
    pub signal_hash: u64,
    pub timestamp_us: u64,
    pub tighten: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bark_event_stack_allocable() {
        // Verify BarkEvent can live on stack
        let event = BarkEvent {
            kind: BarkKind::Detection,
            source_breed: "detector",
            signal_hash: 0xDEAD_BEEF,
            timestamp_us: 1000000,
            tighten: true,
        };

        assert_eq!(event.source_breed, "detector");
        assert!(event.tighten);
    }

    #[test]
    fn test_bark_kind_copy_clone() {
        // Verify BarkKind derives Copy and Clone
        let kind1 = BarkKind::Guard;
        let kind2 = kind1;
        assert!(matches!(kind2, BarkKind::Guard));
    }
}
