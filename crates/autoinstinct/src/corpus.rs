//! Trace corpus ingestion.
//!
//! A `TraceCorpus` is an append-only collection of `Episode`s, each carrying
//! the closed-context surface (snapshot/posture/context fingerprint), the
//! observed `AutonomicInstinct`, and a `urn:blake3` receipt URN.

use serde::{Deserialize, Serialize};

use crate::AutonomicInstinct;

/// Single recorded episode of situated cognition.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Episode {
    /// `urn:blake3:` of the closed-context fingerprint (snapshot + posture
    /// + context bits, NUL-separated, hashed).
    pub context_urn: String,
    /// Observed response class.
    pub response: AutonomicInstinct,
    /// `urn:blake3:` of the receipt material.
    pub receipt_urn: String,
    /// Free-form outcome tag from the runtime monitor (e.g. "earned",
    /// "rolled-back", "user-corrected").
    pub outcome: Option<String>,
}

/// Append-only trace corpus. Episodes are interpreted in insertion order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TraceCorpus {
    /// Episodes in append order.
    pub episodes: Vec<Episode>,
}

impl TraceCorpus {
    /// Empty corpus.
    #[must_use]
    pub const fn new() -> Self {
        Self { episodes: Vec::new() }
    }

    /// Append an episode. Returns the new length.
    pub fn push(&mut self, ep: Episode) -> usize {
        self.episodes.push(ep);
        self.episodes.len()
    }

    /// Number of episodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.episodes.len()
    }

    /// True iff the corpus is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }

    /// Count episodes by response class.
    #[must_use]
    pub fn count_by_response(&self, r: AutonomicInstinct) -> usize {
        self.episodes.iter().filter(|e| e.response == r).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_round_trips_episodes() {
        let mut c = TraceCorpus::new();
        c.push(Episode {
            context_urn: "urn:blake3:00".into(),
            response: AutonomicInstinct::Ask,
            receipt_urn: "urn:blake3:11".into(),
            outcome: Some("earned".into()),
        });
        assert_eq!(c.len(), 1);
        assert_eq!(c.count_by_response(AutonomicInstinct::Ask), 1);
        assert_eq!(c.count_by_response(AutonomicInstinct::Refuse), 0);
    }
}
