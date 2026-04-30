//! Motif discovery — find recurring `(closed-context-fingerprint, response)`
//! patterns in a trace corpus that survive perturbation tests.
//!
//! A motif is a candidate building block for a μ policy. Motif discovery
//! is intentionally simple at this stage: count co-occurrences with a
//! minimum support threshold. Phase 2 will add temporal/causal motifs and
//! integrate Phase-7 trace replay invariants.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::corpus::TraceCorpus;
use crate::AutonomicInstinct;

/// A recurring `(context, response)` motif.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Motif {
    /// Closed-context fingerprint URN.
    pub context_urn: String,
    /// Response class consistently associated with the context.
    pub response: AutonomicInstinct,
    /// Number of corpus episodes that match.
    pub support: u32,
}

/// Result of a motif discovery pass.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Motifs {
    /// Motifs sorted by descending support, then by context_urn for
    /// deterministic output.
    pub motifs: Vec<Motif>,
}

/// Discover motifs whose support is at least `min_support`.
///
/// Deterministic: identical input corpora always produce identical output.
#[must_use]
pub fn discover(corpus: &TraceCorpus, min_support: u32) -> Motifs {
    let mut counts: IndexMap<(String, AutonomicInstinct), u32> = IndexMap::new();
    for ep in &corpus.episodes {
        *counts
            .entry((ep.context_urn.clone(), ep.response))
            .or_insert(0) += 1;
    }
    let mut motifs: Vec<Motif> = counts
        .into_iter()
        .filter(|(_, n)| *n >= min_support)
        .map(|((ctx, r), n)| Motif {
            context_urn: ctx,
            response: r,
            support: n,
        })
        .collect();
    motifs.sort_by(|a, b| {
        b.support
            .cmp(&a.support)
            .then_with(|| a.context_urn.cmp(&b.context_urn))
    });
    Motifs { motifs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Episode;

    fn ep(ctx: &str, r: AutonomicInstinct) -> Episode {
        Episode {
            context_urn: ctx.into(),
            response: r,
            receipt_urn: "urn:blake3:00".into(),
            outcome: Some("earned".into()),
        }
    }

    #[test]
    fn motif_support_counts_co_occurrences() {
        let mut c = TraceCorpus::new();
        for _ in 0..3 {
            c.push(ep("urn:blake3:a", AutonomicInstinct::Ask));
        }
        c.push(ep("urn:blake3:b", AutonomicInstinct::Inspect));
        let m = discover(&c, 2);
        assert_eq!(m.motifs.len(), 1);
        assert_eq!(m.motifs[0].support, 3);
        assert_eq!(m.motifs[0].response, AutonomicInstinct::Ask);
    }

    #[test]
    fn motif_discovery_is_deterministic() {
        let mut c = TraceCorpus::new();
        c.push(ep("urn:blake3:a", AutonomicInstinct::Ask));
        c.push(ep("urn:blake3:b", AutonomicInstinct::Inspect));
        c.push(ep("urn:blake3:a", AutonomicInstinct::Ask));
        let m1 = discover(&c, 1);
        let m2 = discover(&c, 1);
        assert_eq!(m1.motifs, m2.motifs);
    }
}
