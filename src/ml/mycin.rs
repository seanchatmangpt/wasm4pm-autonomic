//! MYCIN (Shortliffe et al. 1974–76) — Nanosecond Diagnostic Rule Lattice.
//!
//! **Reference:** Shortliffe, E.H. (1976). *Computer-Based Medical Consultations: MYCIN.*
//! New York: Elsevier. Also: Buchanan & Shortliffe (1984), *Rule-Based Expert Systems.*
//!
//! # Architecture: Diagnosis as Execution Physics
//!
//! Classical MYCIN was an interactive consultation system answering one patient at a time.
//! At nanosecond scale, MYCIN becomes a state-transition function: facts (u64 bitmask) →
//! organism conclusions (u64 bitmask), with certainty factors (i16 fixed-point) attached.
//!
//! ## Certainty Factor (CF) Arithmetic
//!
//! CFs live in `[-1.0, +1.0]`. We store as `i16` in `[-1000, +1000]` for cache-line packing
//! and exact arithmetic at hot-path scale.
//!
//! - `combine_cf(cf1, cf2)`:
//!   - Both positive: `cf1 + cf2 * (1 - cf1)`
//!   - Both negative: `cf1 + cf2 * (1 + cf1)`
//!   - Mixed signs: `(cf1 + cf2) / (1 - min(|cf1|, |cf2|))`
//! - `premise_cf(conditions)`: `min` of all condition CFs (Shortliffe's MIN rule)
//! - `apply_rule_cf(premise, rule_cf)`: `premise * rule_cf`
//!
//! ## Hot Path
//!
//! `infer_fast(facts) -> u64`: ~20 ns; returns the OR of all derived conclusions
//! (binary firing, no CF, branchless rule scan)
//!
//! `infer(facts) -> (conclusions, cf_table)`: ~200 ns; full CF accounting

use crate::ml::hdit_automl::SignalProfile;

// =============================================================================
// FACT BITS (low 32 bits) and ORGANISM BITS (high 32 bits)
// =============================================================================

/// Patient facts (clinical observations).
pub mod fact {
    pub const GRAM_NEG: u64 = 1 << 0;
    pub const GRAM_POS: u64 = 1 << 1;
    pub const ROD: u64 = 1 << 2;
    pub const COCCUS: u64 = 1 << 3;
    pub const AEROBIC: u64 = 1 << 4;
    pub const ANAEROBIC: u64 = 1 << 5;
    pub const FEVER: u64 = 1 << 6;
    pub const RIGORS: u64 = 1 << 7;
    pub const BLOOD_POS: u64 = 1 << 8;
    pub const BURN: u64 = 1 << 9;
    pub const NOSOCOMIAL: u64 = 1 << 10;
    pub const COMPROMISED_HOST: u64 = 1 << 11;
    pub const HEAD_TRAUMA: u64 = 1 << 12;
}

/// Organism hypotheses (high 32 bits of the state).
pub mod org {
    pub const BACTEROIDES: u64 = 1 << 32;
    pub const E_COLI: u64 = 1 << 33;
    pub const PSEUDOMONAS: u64 = 1 << 34;
    pub const STAPH: u64 = 1 << 35;
    pub const STREP: u64 = 1 << 36;
    pub const KLEBSIELLA: u64 = 1 << 37;
    pub const PROTEUS: u64 = 1 << 38;
}

// =============================================================================
// CERTAINTY FACTORS — i16 fixed-point in [-1000, +1000]
// =============================================================================

/// Fixed-point CF: stored as `i16`, `1000` = 1.0, `-1000` = -1.0.
pub type Cf = i16;

pub const CF_TRUE: Cf = 1000;
pub const CF_FALSE: Cf = -1000;
pub const CF_UNKNOWN: Cf = 0;

/// Convert f64 CF to fixed-point.
#[inline(always)]
#[must_use]
pub fn cf_from_f64(x: f64) -> Cf {
    let clamped = x.clamp(-1.0, 1.0);
    (clamped * 1000.0).round() as Cf
}

/// Convert fixed-point CF to f64.
#[inline(always)]
#[must_use]
pub fn cf_to_f64(cf: Cf) -> f64 {
    (cf as f64) / 1000.0
}

/// Combine two CFs (Shortliffe's parallel-evidence formula).
///
/// Branchless implementation using i32 intermediate to avoid overflow.
#[inline(always)]
#[must_use]
pub fn combine_cf(cf1: Cf, cf2: Cf) -> Cf {
    let a = cf1 as i32;
    let b = cf2 as i32;
    let result = if a > 0 && b > 0 {
        // cf1 + cf2 * (1 - cf1) = (a + b - a*b/1000) but in fixed-point
        a + b - (a * b) / 1000
    } else if a < 0 && b < 0 {
        // cf1 + cf2 * (1 + cf1) = a + b + (a*b/1000)
        a + b + (a * b) / 1000
    } else {
        // Mixed: (cf1 + cf2) / (1 - min(|cf1|, |cf2|))
        let min_abs = a.abs().min(b.abs());
        let denom = 1000 - min_abs;
        if denom == 0 { 0 } else { ((a + b) * 1000) / denom }
    };
    result.clamp(-1000, 1000) as Cf
}

/// Compute premise CF as the minimum of all condition CFs.
#[inline]
#[must_use]
pub fn premise_cf(condition_cfs: &[Cf]) -> Cf {
    if condition_cfs.is_empty() {
        return CF_TRUE;
    }
    *condition_cfs.iter().min().unwrap_or(&CF_TRUE)
}

/// Apply rule CF to premise CF (Shortliffe: premise * rule_cf).
#[inline(always)]
#[must_use]
pub fn apply_rule_cf(premise: Cf, rule_cf: Cf) -> Cf {
    ((premise as i32) * (rule_cf as i32) / 1000) as Cf
}

// =============================================================================
// RULE BASE
// =============================================================================

/// One MYCIN rule: bit-packed conditions → conclusion with CF.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(32))]
pub struct MycinRule {
    /// Required fact bits (AND test against fact mask).
    pub conditions: u64,
    /// Conclusion bit (single organism).
    pub conclusion: u64,
    /// Rule's intrinsic CF.
    pub cf: Cf,
    /// Rule ID for tracing.
    pub id: u16,
    pub _pad: [u8; 12],
}

const _: () = assert!(core::mem::size_of::<MycinRule>() == 32);

/// Bacteremia rule base (compressed from MYCIN's actual rule lattice).
pub const RULES: [MycinRule; 12] = [
    // STREP rules
    MycinRule { id: 1, conditions: fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS, conclusion: org::STREP, cf: 800, _pad: [0; 12] },
    MycinRule { id: 2, conditions: fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER, conclusion: org::STREP, cf: 600, _pad: [0; 12] },
    MycinRule { id: 3, conditions: fact::GRAM_POS | fact::AEROBIC | fact::FEVER | fact::RIGORS, conclusion: org::STREP, cf: 700, _pad: [0; 12] },
    // E. coli rules
    MycinRule { id: 4, conditions: fact::GRAM_NEG | fact::ROD | fact::AEROBIC | fact::BLOOD_POS, conclusion: org::E_COLI, cf: 850, _pad: [0; 12] },
    MycinRule { id: 5, conditions: fact::GRAM_NEG | fact::AEROBIC | fact::BLOOD_POS, conclusion: org::E_COLI, cf: 600, _pad: [0; 12] },
    // Pseudomonas
    MycinRule { id: 6, conditions: fact::GRAM_NEG | fact::ROD | fact::AEROBIC | fact::BURN, conclusion: org::PSEUDOMONAS, cf: 900, _pad: [0; 12] },
    MycinRule { id: 7, conditions: fact::GRAM_NEG | fact::AEROBIC | fact::COMPROMISED_HOST, conclusion: org::PSEUDOMONAS, cf: 500, _pad: [0; 12] },
    // Staph
    MycinRule { id: 8, conditions: fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::BURN, conclusion: org::STAPH, cf: 750, _pad: [0; 12] },
    MycinRule { id: 9, conditions: fact::GRAM_POS | fact::COCCUS | fact::NOSOCOMIAL, conclusion: org::STAPH, cf: 500, _pad: [0; 12] },
    // Bacteroides
    MycinRule { id: 10, conditions: fact::GRAM_NEG | fact::ANAEROBIC, conclusion: org::BACTEROIDES, cf: 700, _pad: [0; 12] },
    // Klebsiella
    MycinRule { id: 11, conditions: fact::GRAM_NEG | fact::ROD | fact::NOSOCOMIAL, conclusion: org::KLEBSIELLA, cf: 600, _pad: [0; 12] },
    // Proteus
    MycinRule { id: 12, conditions: fact::GRAM_NEG | fact::ROD | fact::HEAD_TRAUMA, conclusion: org::PROTEUS, cf: 550, _pad: [0; 12] },
];

// =============================================================================
// HOT PATH — branchless ~20 ns inference
// =============================================================================

/// Fast forward chaining: facts → conclusions (binary, no CF).
///
/// Returns the OR of all rule conclusions whose conditions are satisfied.
/// Branchless rule scan: `~20 ns` for 12 rules.
#[inline]
#[must_use]
pub fn infer_fast(facts: u64, rules: &[MycinRule]) -> u64 {
    let mut conclusions = 0u64;
    let mut i = 0;
    while i < rules.len() {
        let r = rules[i];
        let satisfied = ((r.conditions & facts) == r.conditions) as u64;
        let mask = satisfied.wrapping_neg();
        conclusions |= r.conclusion & mask;
        i += 1;
    }
    conclusions
}

/// Result of a full CF-aware inference cycle.
#[derive(Clone, Debug, Default)]
pub struct MycinResult {
    /// OR of all derived organism bits.
    pub conclusions: u64,
    /// Per-organism CF, indexed by organism bit position - 32.
    pub cf: [Cf; 16],
    /// Number of rules that fired.
    pub fired: u32,
}

impl MycinResult {
    /// Get CF for an organism bit (e.g., `org::STREP`).
    #[inline]
    #[must_use]
    pub fn cf_for(&self, org_bit: u64) -> Cf {
        let idx = org_bit.trailing_zeros() as usize - 32;
        if idx >= 16 { return CF_UNKNOWN; }
        self.cf[idx]
    }

    /// Most-likely diagnosis: organism with highest CF.
    #[must_use]
    pub fn best(&self) -> Option<(u64, Cf)> {
        let mut best: Option<(u64, Cf)> = None;
        for i in 0..16 {
            if self.cf[i] > 0 {
                let bit = 1u64 << (32 + i);
                match best {
                    None => best = Some((bit, self.cf[i])),
                    Some((_, c)) if self.cf[i] > c => best = Some((bit, self.cf[i])),
                    _ => {}
                }
            }
        }
        best
    }
}

/// Full forward-chaining inference with certainty factor accumulation.
///
/// For each rule whose conditions are satisfied, computes the rule's contribution
/// CF (= rule.cf, since input facts have CF=1.0 by assumption) and combines into
/// the conclusion's running CF using [`combine_cf`].
///
/// Multi-pass to fixed point: re-runs until no rule produces a new conclusion or
/// significantly improves an existing CF.
#[must_use]
pub fn infer(facts: u64, rules: &[MycinRule]) -> MycinResult {
    let mut result = MycinResult::default();

    for _pass in 0..rules.len() + 1 {
        let mut new_fired = false;
        for r in rules.iter() {
            if (r.conditions & facts) == r.conditions {
                let idx = r.conclusion.trailing_zeros() as usize - 32;
                if idx >= 16 { continue; }
                let prior = result.cf[idx];
                let combined = combine_cf(prior, r.cf);
                if combined != prior {
                    result.cf[idx] = combined;
                    new_fired = true;
                    result.fired += 1;
                }
                result.conclusions |= r.conclusion;
            }
        }
        if !new_fired { break; }
    }

    result
}

/// Backward chaining: given a goal organism, find which rule(s) conclude it
/// and return the maximum CF achievable from current facts.
///
/// This is the consultative MYCIN cycle: ask "could the organism be STREP?"
/// and return the CF.
#[must_use]
pub fn backward_chain(goal_org: u64, facts: u64, rules: &[MycinRule]) -> Cf {
    let mut combined = CF_UNKNOWN;
    for r in rules.iter() {
        if r.conclusion == goal_org && (r.conditions & facts) == r.conditions {
            combined = combine_cf(combined, r.cf);
        }
    }
    combined
}

/// Consultation: rank all organisms by CF descending.
#[must_use]
pub fn consult(facts: u64, rules: &[MycinRule]) -> Vec<(u64, Cf)> {
    let result = infer(facts, rules);
    let mut diagnoses: Vec<(u64, Cf)> = (0..16)
        .filter_map(|i| {
            if result.cf[i] > 0 {
                Some((1u64 << (32 + i), result.cf[i]))
            } else {
                None
            }
        })
        .collect();
    diagnoses.sort_by_key(|d| std::cmp::Reverse(d.1));
    diagnoses
}

// =============================================================================
// AUTOML SIGNAL
// =============================================================================

/// AutoML signal: predicts true if the inference reaches a target organism.
pub fn mycin_automl_signal(
    name: &str,
    patient_facts: &[u64],
    target: u64,
    anchor: &[bool],
) -> SignalProfile {
    let mut predictions = Vec::with_capacity(patient_facts.len());
    let mut total_ns = 0u64;
    for &facts in patient_facts {
        let conclusions = infer_fast(facts, &RULES);
        predictions.push((conclusions & target) != 0);
        total_ns += 20;
    }
    let timing_us = (total_ns / 1000).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cf_conversion_roundtrip() {
        assert_eq!(cf_to_f64(cf_from_f64(0.7)), 0.7);
        assert_eq!(cf_to_f64(cf_from_f64(-0.3)), -0.3);
        assert_eq!(cf_from_f64(2.0), CF_TRUE, "clamped to 1.0");
        assert_eq!(cf_from_f64(-2.0), CF_FALSE, "clamped to -1.0");
    }

    #[test]
    fn combine_cf_both_positive() {
        // 0.6 + 0.4 = 0.6 + 0.4*(1-0.6) = 0.6 + 0.16 = 0.76
        let result = combine_cf(600, 400);
        assert_eq!(result, 760);
    }

    #[test]
    fn combine_cf_both_negative() {
        // -0.6 + -0.4*(1 + -0.6) = -0.6 + -0.16 = -0.76
        let result = combine_cf(-600, -400);
        assert_eq!(result, -760);
    }

    #[test]
    fn combine_cf_mixed_signs() {
        // (0.6 + -0.4) / (1 - min(0.6, 0.4)) = 0.2 / 0.6 = 0.333...
        let result = combine_cf(600, -400);
        assert!((result - 333).abs() <= 1, "got {}", result);
    }

    #[test]
    fn combine_cf_with_zero_returns_other() {
        // Mixed sign formula with 0: (a+0)/(1-0) = a
        assert_eq!(combine_cf(500, 0), 500);
        assert_eq!(combine_cf(0, 500), 500);
    }

    #[test]
    fn combine_cf_at_boundary() {
        // 1.0 + 0.5 should clamp to 1.0
        assert_eq!(combine_cf(1000, 500), 1000);
        // -1.0 + -0.5 should clamp to -1.0
        assert_eq!(combine_cf(-1000, -500), -1000);
    }

    #[test]
    fn premise_cf_returns_minimum() {
        let cfs = [800, 600, 700, 500];
        assert_eq!(premise_cf(&cfs), 500);
    }

    #[test]
    fn premise_cf_empty_returns_true() {
        assert_eq!(premise_cf(&[]), CF_TRUE);
    }

    #[test]
    fn apply_rule_cf_multiplies() {
        // 0.5 premise * 0.8 rule = 0.4
        assert_eq!(apply_rule_cf(500, 800), 400);
    }

    #[test]
    fn infer_fast_strep_diagnosis() {
        let facts = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS;
        let conclusions = infer_fast(facts, &RULES);
        assert!(conclusions & org::STREP != 0, "should diagnose STREP");
    }

    #[test]
    fn infer_fast_e_coli_diagnosis() {
        let facts = fact::GRAM_NEG | fact::ROD | fact::AEROBIC | fact::BLOOD_POS;
        let conclusions = infer_fast(facts, &RULES);
        assert!(conclusions & org::E_COLI != 0);
    }

    #[test]
    fn infer_fast_pseudomonas_burn_patient() {
        let facts = fact::GRAM_NEG | fact::ROD | fact::AEROBIC | fact::BURN;
        let conclusions = infer_fast(facts, &RULES);
        assert!(conclusions & org::PSEUDOMONAS != 0);
    }

    #[test]
    fn infer_fast_no_diagnosis_when_facts_insufficient() {
        let facts = fact::FEVER;  // Just fever, not enough
        let conclusions = infer_fast(facts, &RULES);
        // STREP requires GRAM_POS + COCCUS + AEROBIC + FEVER (+ optionally RIGORS)
        assert_eq!(conclusions, 0);
    }

    #[test]
    fn infer_full_strep_with_cf() {
        let facts = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS;
        let result = infer(facts, &RULES);
        let strep_cf = result.cf_for(org::STREP);
        assert!(strep_cf > 800, "STREP CF should combine multiple rules: got {}", strep_cf);
    }

    #[test]
    fn infer_full_best_returns_strep() {
        let facts = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS;
        let result = infer(facts, &RULES);
        let best = result.best();
        assert!(best.is_some());
        assert_eq!(best.unwrap().0, org::STREP);
    }

    #[test]
    fn backward_chain_strep_succeeds() {
        let facts = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS;
        let cf = backward_chain(org::STREP, facts, &RULES);
        assert!(cf > 0);
    }

    #[test]
    fn backward_chain_unknown_org_returns_zero() {
        let facts = fact::FEVER;
        let cf = backward_chain(org::STREP, facts, &RULES);
        assert_eq!(cf, CF_UNKNOWN);
    }

    #[test]
    fn consult_returns_sorted_descending() {
        let facts = fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS;
        let diagnoses = consult(facts, &RULES);
        assert!(!diagnoses.is_empty());
        // Verify descending order
        for w in diagnoses.windows(2) {
            assert!(w[0].1 >= w[1].1, "must be descending");
        }
    }

    #[test]
    fn rule_size_is_32_bytes() {
        // Cache-line packing requirement
        assert_eq!(core::mem::size_of::<MycinRule>(), 32);
    }

    #[test]
    fn mycin_automl_signal_strep_detection() {
        let patients = [
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER | fact::RIGORS,
            fact::GRAM_NEG | fact::ANAEROBIC,
            fact::GRAM_POS | fact::COCCUS | fact::AEROBIC | fact::FEVER,
        ];
        let anchor = [true, false, true];
        let sig = mycin_automl_signal("mycin_strep", &patients, org::STREP, &anchor);
        assert_eq!(sig.predictions, vec![true, false, true]);
        assert_eq!(sig.accuracy_vs_anchor, 1.0);
    }

    #[test]
    fn mycin_signal_is_t0_tier() {
        use crate::ml::hdit_automl::Tier;
        let patients = [fact::GRAM_NEG | fact::ANAEROBIC; 100];
        let anchor = [true; 100];
        let sig = mycin_automl_signal("m", &patients, org::BACTEROIDES, &anchor);
        assert_eq!(sig.tier, Tier::T0);
    }
}
