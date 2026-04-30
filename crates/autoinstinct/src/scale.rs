//! Phase 10 — Continuous learning at scale.
//!
//! Federated motif aggregation across tenants without raw-trace sharing,
//! plus differential-privacy noise injection on aggregate counts and SLO
//! tracking on aggregate response shape. Tenants submit local
//! `MotifSummary`s; the aggregator merges them under DP-Laplace noise and
//! produces a global motif distribution.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::motifs::Motifs;
use crate::AutonomicInstinct;

/// Per-tenant submission (privacy-preserving).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MotifSummary {
    /// Tenant identifier (opaque).
    pub tenant: String,
    /// `(context_urn, response, count)` triples.
    pub counts: Vec<(String, AutonomicInstinct, u32)>,
    /// Total observations contributing to this summary.
    pub total: u64,
}

/// Build a tenant summary from local motifs.
#[must_use]
pub fn summarize(tenant: &str, motifs: &Motifs) -> MotifSummary {
    let counts: Vec<(String, AutonomicInstinct, u32)> = motifs
        .motifs
        .iter()
        .map(|m| (m.context_urn.clone(), m.response, m.support))
        .collect();
    let total: u64 = counts.iter().map(|(_, _, n)| *n as u64).sum();
    MotifSummary {
        tenant: tenant.to_string(),
        counts,
        total,
    }
}

/// Differential-privacy parameters (Laplace mechanism).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DpParams {
    /// Sensitivity (per-record contribution bound).
    pub sensitivity: f64,
    /// Privacy budget. Smaller = more privacy, more noise.
    pub epsilon: f64,
    /// Deterministic seed for the noise RNG (so federated aggregation is
    /// reproducible at audit time).
    pub seed: u64,
}

impl DpParams {
    /// Conservative defaults: sensitivity = 1.0, epsilon = 1.0.
    #[must_use]
    pub fn default_with_seed(seed: u64) -> Self {
        Self {
            sensitivity: 1.0,
            epsilon: 1.0,
            seed,
        }
    }
}

/// Deterministic Laplace noise generator (seeded BLAKE3 / sign-folded).
struct LaplaceRng {
    state: [u8; 32],
    scale: f64,
}

impl LaplaceRng {
    fn new(seed: u64, scale: f64) -> Self {
        let mut s = [0u8; 32];
        s[..8].copy_from_slice(&seed.to_le_bytes());
        Self {
            state: *blake3::hash(&s).as_bytes(),
            scale,
        }
    }
    fn step(&mut self) -> f64 {
        self.state = *blake3::hash(&self.state).as_bytes();
        // Map two u32s to (-1, 1) Laplace using inverse CDF.
        let u1 = u32::from_le_bytes(self.state[..4].try_into().unwrap()) as f64 / u32::MAX as f64;
        // Inverse Laplace CDF.
        let centered = u1 - 0.5;
        let sign = if centered < 0.0 { -1.0 } else { 1.0 };
        -self.scale * sign * (1.0f64 - 2.0 * centered.abs()).max(1e-9).ln()
    }
}

/// Aggregate per-tenant summaries with differential-privacy noise.
#[must_use]
pub fn aggregate(summaries: &[MotifSummary], dp: DpParams) -> Vec<(String, AutonomicInstinct, f64)> {
    let mut counts: IndexMap<(String, AutonomicInstinct), f64> = IndexMap::new();
    for s in summaries {
        for (ctx, r, n) in &s.counts {
            *counts.entry((ctx.clone(), *r)).or_insert(0.0) += *n as f64;
        }
    }
    let scale = dp.sensitivity / dp.epsilon.max(1e-9);
    let mut rng = LaplaceRng::new(dp.seed, scale);
    counts
        .into_iter()
        .map(|((ctx, r), n)| (ctx, r, (n + rng.step()).max(0.0)))
        .collect()
}

/// SLO descriptor.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Slo {
    /// Maximum permissible drift mismatch rate.
    pub max_drift_rate: f64,
    /// Minimum admitted-policy fitness across the federated corpus.
    pub min_fitness: f64,
}

/// Verdict over an SLO.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SloVerdict {
    /// True iff drift is within budget.
    pub drift_ok: bool,
    /// True iff fitness is above the floor.
    pub fitness_ok: bool,
}

impl SloVerdict {
    /// True iff every SLO held.
    #[must_use]
    pub fn all_ok(&self) -> bool {
        self.drift_ok && self.fitness_ok
    }
}

/// Evaluate an SLO against measured drift and fitness.
#[must_use]
pub fn evaluate(slo: Slo, drift_rate: f64, fitness: f64) -> SloVerdict {
    SloVerdict {
        drift_ok: drift_rate <= slo.max_drift_rate,
        fitness_ok: fitness >= slo.min_fitness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motifs::Motif;

    fn summary(tenant: &str, ctx: &str, r: AutonomicInstinct, n: u32) -> MotifSummary {
        let m = Motifs {
            motifs: vec![Motif {
                context_urn: ctx.into(),
                response: r,
                support: n,
            }],
        };
        summarize(tenant, &m)
    }

    #[test]
    fn aggregate_sums_counts_across_tenants() {
        let s1 = summary("t1", "urn:blake3:a", AutonomicInstinct::Ask, 10);
        let s2 = summary("t2", "urn:blake3:a", AutonomicInstinct::Ask, 7);
        // Use very small scale → ~deterministic summation.
        let dp = DpParams {
            sensitivity: 1.0,
            epsilon: 1e6,
            seed: 42,
        };
        let agg = aggregate(&[s1, s2], dp);
        let entry = agg
            .iter()
            .find(|(c, r, _)| c == "urn:blake3:a" && *r == AutonomicInstinct::Ask)
            .unwrap();
        assert!((entry.2 - 17.0).abs() < 1.0, "{}", entry.2);
    }

    #[test]
    fn aggregate_is_deterministic_under_fixed_seed() {
        let s1 = summary("t1", "urn:blake3:a", AutonomicInstinct::Ask, 5);
        let dp = DpParams::default_with_seed(99);
        let a = aggregate(&[s1.clone()], dp);
        let b = aggregate(&[s1], dp);
        assert_eq!(a, b);
    }

    #[test]
    fn slo_passes_within_budget() {
        let slo = Slo {
            max_drift_rate: 0.1,
            min_fitness: 0.9,
        };
        assert!(evaluate(slo, 0.05, 0.95).all_ok());
    }

    #[test]
    fn slo_fails_when_drift_or_fitness_breaks() {
        let slo = Slo {
            max_drift_rate: 0.1,
            min_fitness: 0.9,
        };
        assert!(!evaluate(slo, 0.5, 0.95).drift_ok);
        assert!(!evaluate(slo, 0.05, 0.5).fitness_ok);
    }
}
