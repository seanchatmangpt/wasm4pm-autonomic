//! Phase 3 — Causal & counterfactual engine.
//!
//! Estimates the *causal effect* of a context bit on the response class
//! using simple do-calculus-style intervention. For each context-URN
//! component we compare the response distribution with-vs-without that
//! component and report the absolute response shift. This is the
//! anti-correlation-only signal that lets the gauntlet distinguish
//! load-bearing context from coincidence.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::corpus::TraceCorpus;
use crate::AutonomicInstinct;

/// Causal effect of a single context-component on response distribution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CausalEffect {
    /// Context component (substring of the context_urn — e.g. a bit tag).
    pub component: String,
    /// Response distribution when the component is present.
    pub with: Vec<(AutonomicInstinct, u32)>,
    /// Response distribution when the component is absent.
    pub without: Vec<(AutonomicInstinct, u32)>,
    /// L1 distance between the two distributions, normalized to [0, 1].
    pub effect_size: f64,
}

/// Estimate the causal effect of `component` on the response distribution
/// in `corpus`. `component` is treated as a substring marker on
/// `context_urn`; episodes whose context_urn contains it are the "with"
/// arm, others are "without".
#[must_use]
pub fn effect(corpus: &TraceCorpus, component: &str) -> CausalEffect {
    let mut with: IndexMap<AutonomicInstinct, u32> = IndexMap::new();
    let mut without: IndexMap<AutonomicInstinct, u32> = IndexMap::new();
    let mut total_with = 0u32;
    let mut total_without = 0u32;
    for ep in &corpus.episodes {
        let bucket = if ep.context_urn.contains(component) {
            total_with += 1;
            &mut with
        } else {
            total_without += 1;
            &mut without
        };
        *bucket.entry(ep.response).or_insert(0) += 1;
    }
    let normalize = |m: &IndexMap<AutonomicInstinct, u32>, total: u32| -> Vec<(AutonomicInstinct, f64)> {
        if total == 0 {
            return Vec::new();
        }
        m.iter()
            .map(|(r, n)| (*r, *n as f64 / total as f64))
            .collect()
    };
    let p_with = normalize(&with, total_with);
    let p_without = normalize(&without, total_without);
    // L1 distance in (0, 2); normalize to (0, 1).
    let mut effect_size = 0.0;
    for r in [
        AutonomicInstinct::Settle,
        AutonomicInstinct::Retrieve,
        AutonomicInstinct::Inspect,
        AutonomicInstinct::Ask,
        AutonomicInstinct::Refuse,
        AutonomicInstinct::Escalate,
        AutonomicInstinct::Ignore,
    ] {
        let pw = p_with.iter().find(|(c, _)| *c == r).map(|(_, p)| *p).unwrap_or(0.0);
        let po = p_without
            .iter()
            .find(|(c, _)| *c == r)
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        effect_size += (pw - po).abs();
    }
    effect_size /= 2.0;
    CausalEffect {
        component: component.to_string(),
        with: with.into_iter().collect(),
        without: without.into_iter().collect(),
        effect_size,
    }
}

/// Rank a list of components by absolute causal effect (descending).
#[must_use]
pub fn rank(corpus: &TraceCorpus, components: &[&str]) -> Vec<CausalEffect> {
    let mut effects: Vec<CausalEffect> = components.iter().map(|c| effect(corpus, c)).collect();
    effects.sort_by(|a, b| b.effect_size.partial_cmp(&a.effect_size).unwrap_or(std::cmp::Ordering::Equal));
    effects
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
    fn fatigue_component_has_high_effect_when_response_diverges() {
        let mut c = TraceCorpus::new();
        for _ in 0..10 {
            c.push(ep("urn:blake3:fatigue+routine", AutonomicInstinct::Ask));
        }
        for _ in 0..10 {
            c.push(ep("urn:blake3:routine", AutonomicInstinct::Inspect));
        }
        let e = effect(&c, "fatigue");
        assert!(e.effect_size > 0.8, "got {}", e.effect_size);
    }

    #[test]
    fn irrelevant_component_has_low_effect() {
        let mut c = TraceCorpus::new();
        for _ in 0..10 {
            c.push(ep("urn:blake3:routine", AutonomicInstinct::Ask));
        }
        for _ in 0..10 {
            c.push(ep("urn:blake3:routine-other", AutonomicInstinct::Ask));
        }
        let e = effect(&c, "other");
        assert!(e.effect_size < 0.1, "got {}", e.effect_size);
    }

    #[test]
    fn rank_orders_by_effect_size() {
        let mut c = TraceCorpus::new();
        for _ in 0..10 {
            c.push(ep("urn:blake3:fatigue+x", AutonomicInstinct::Ask));
        }
        for _ in 0..10 {
            c.push(ep("urn:blake3:other-x", AutonomicInstinct::Inspect));
        }
        let r = rank(&c, &["fatigue", "x"]);
        assert_eq!(r[0].component, "fatigue");
    }
}
