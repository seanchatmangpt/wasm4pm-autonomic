//! Result Re-entry & Normalization (Phase 4).
//!
//! Converts external results (MCP, A2A) into bounded COG8 constructs
//! or routes overflow to cold storage.

use crate::construct8::{Construct8, Triple};

/// Result from an MCP operation.
///
/// Contains the raw triples harvested from an MCP server response.
#[derive(Debug, Clone)]
pub struct MCPResult {
    /// Raw triples produced by the MCP operation.
    pub triples: Vec<Triple>,
}

/// Artifact produced by an A2A task.
#[derive(Debug, Clone)]
pub struct A2AArtifact {
    /// Triples harvested from the agent's work.
    pub triples: Vec<Triple>,
}

/// Evidence that exceeds the hot-path limit (8 triples).
///
/// Routed to durable storage/ledger instead of immediate field materialization.
#[derive(Debug, Clone)]
pub struct ColdEvidence {
    /// All unique triples produced by the operation.
    pub triples: Vec<Triple>,
}

/// Outcome of a result re-entry attempt.
#[derive(Debug, Clone)]
pub enum ReentryOutcome {
    /// Result fits in the 8-triple hot path.
    Hot(Construct8),
    /// Result is too large and was routed to cold storage.
    Cold(ColdEvidence),
}

/// Projector that transforms an [`MCPResult`] into a [`ReentryOutcome`].
pub trait ResultProjector {
    /// Projects the MCP result into a [`ReentryOutcome`].
    fn project(&self, result: &MCPResult) -> ReentryOutcome;
}

/// Re-entry logic for [`A2AArtifact`]s.
pub trait ArtifactReentry {
    /// Normalizes and converts an artifact into a [`ReentryOutcome`].
    fn reenter(&self, artifact: &A2AArtifact) -> ReentryOutcome;
}

/// Default implementation of re-entry logic.
///
/// Strictly enforces the 8-triple limit and deduplicates triples.
pub struct DefaultReentry;

impl ResultProjector for DefaultReentry {
    fn project(&self, result: &MCPResult) -> ReentryOutcome {
        normalize_to_outcome(&result.triples)
    }
}

impl ArtifactReentry for DefaultReentry {
    fn reenter(&self, artifact: &A2AArtifact) -> ReentryOutcome {
        normalize_to_outcome(&artifact.triples)
    }
}

/// Normalizes a set of triples and converts them into a [`ReentryOutcome`].
///
/// 1. Deduplicates input triples.
/// 2. Validates against the 8-triple hot-path budget.
/// 3. Returns [`ReentryOutcome::Hot`] if within budget, else [`ReentryOutcome::Cold`].
fn normalize_to_outcome(triples: &[Triple]) -> ReentryOutcome {
    // Deduplication (Normalization)
    let mut unique = Vec::with_capacity(triples.len().min(16));
    for triple in triples {
        if !unique.contains(triple) {
            unique.push(*triple);
        }
    }

    if unique.len() > 8 {
        return ReentryOutcome::Cold(ColdEvidence { triples: unique });
    }

    let mut construct = Construct8::empty();
    for triple in unique {
        if !construct.push(triple) {
            // Defensive: should be unreachable due to check above.
            return ReentryOutcome::Cold(ColdEvidence {
                triples: vec![triple],
            });
        }
    }

    ReentryOutcome::Hot(construct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct8::{ObjectId, PredicateId};

    fn make_triple(i: u32) -> Triple {
        Triple::new(ObjectId(i), PredicateId(i as u16), ObjectId(i))
    }

    #[test]
    fn test_hot_reentry() {
        let reentry = DefaultReentry;
        let triples = vec![make_triple(1), make_triple(2)];
        let artifact = A2AArtifact { triples };

        let outcome = reentry.reenter(&artifact);
        match outcome {
            ReentryOutcome::Hot(delta) => assert_eq!(delta.len(), 2),
            _ => panic!("Expected Hot outcome"),
        }
    }

    #[test]
    fn test_cold_reentry_overflow() {
        let reentry = DefaultReentry;
        let mut triples = Vec::new();
        for i in 0..9 {
            triples.push(make_triple(i));
        }
        let artifact = A2AArtifact { triples };

        let outcome = reentry.reenter(&artifact);
        match outcome {
            ReentryOutcome::Cold(evidence) => assert_eq!(evidence.triples.len(), 9),
            _ => panic!("Expected Cold outcome"),
        }
    }

    #[test]
    fn test_deduplication() {
        let reentry = DefaultReentry;
        let triples = vec![make_triple(1), make_triple(1), make_triple(1)];
        let artifact = A2AArtifact { triples };

        let outcome = reentry.reenter(&artifact);
        match outcome {
            ReentryOutcome::Hot(delta) => assert_eq!(delta.len(), 1),
            _ => panic!("Expected Hot outcome"),
        }
    }
}
