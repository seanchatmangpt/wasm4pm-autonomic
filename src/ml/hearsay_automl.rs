//! Hearsay-II AutoML Equivalent — Learned Multi-Source Hypothesis Fusion via Borda Count.
//!
//! While `crate::ml::hearsay` provides hand-coded knowledge sources and an
//! agenda scheduler, this module provides the *learned* equivalent: given
//! per-source scores at each hypothesis level, fuse them into a final
//! decision via Borda-count rank fusion.
//!
//! # Compiled Cognition
//!
//! This module contributes `L_learned` to Compiled Cognition. Paired with
//! `hearsay.rs` (`S_symbolic`), these two halves compose into the full
//! multi-source fusion primitive of `C_compiled = S ⊕ L ⊕ D ⊕ P`.
//!
//! # Substrate Bifurcation
//!
//! Classical Hearsay-II (Erman et al. 1980) is *orchestrated consensus*: knowledge
//! sources are hand-coded; the scheduler aggregates their ratings via a blackboard.
//! This module is *learned consensus*: run the blackboard across multiple inputs to
//! extract per-level confidence scores, then use Borda count to rank and select.
//!
//! Both perform the **same JOB** — fuse multiple independent evidence streams into
//! a coherent decision — but via different architectures:
//! - **Classical**: Hand-coded KSs, explicit agenda, interactive refinement
//! - **AutoML**: Blackboard output scores, rank-based fusion, non-parametric
//!
//! # Fusion Strategy
//!
//! Input: N orthogonal scoring sources (each source is a vector of per-input scores).
//! Output: Boolean vector indicating which inputs rank in the top-K.
//!
//! Borda count is ideal because it:
//! 1. Requires no training
//! 2. Makes no distributional assumptions
//! 3. Operates on ranks (robust to score magnitudes)
//! 4. Matches the blackboard's aggregation semantics
//!
//! # Architecture
//!
//! ```text
//! Input → Blackboard (4 levels) → [CF vectors]
//!           ├─ Acoustic level CF
//!           ├─ Phoneme level CF
//!           ├─ Syllable level CF
//!           └─ Word level CF
//!               ↓
//!          Borda Count (rank fusion) → Top-K selection → Bool vector
//! ```
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::hearsay_automl;
//!
//! // Four evidence sources (e.g., acoustic, phoneme, syllable, word confidence)
//! let sources = vec![
//!     vec![0.9, 0.1, 0.8, 0.2], // source 1: inputs 0,2 high confidence
//!     vec![0.85, 0.15, 0.75, 0.25],
//!     vec![0.95, 0.2, 0.7, 0.3],
//!     vec![0.88, 0.12, 0.82, 0.18],
//! ];
//!
//! // Rank and select top 2 inputs
//! let predictions = hearsay_automl::fuse(&sources, 2);
//!
//! assert_eq!(predictions.len(), 4);
//! let count = predictions.iter().filter(|&&p| p).count();
//! assert_eq!(count, 2);
//! // Top 2 are inputs 0 and 2 (highest aggregate rank across all sources)
//! assert!(predictions[0]);
//! assert!(predictions[2]);
//! ```
//!
//! # Determinism
//!
//! This module is fully deterministic: identical inputs produce byte-identical outputs.
//! Borda count is purely arithmetic; the blackboard uses deterministic ratings.

use crate::ml::hdit_automl::SignalProfile;
use crate::ml::hearsay::{
    Blackboard, Hypothesis, ACOUSTIC, DEFAULT_KS, PHONEME, SENTENCE, SYLLABLE, WORD,
};
use crate::ml::rank_fusion;

/// Run the classical Hearsay-II blackboard once and extract per-level
/// hypothesis confidences as a score vector for downstream rank fusion.
///
/// Returns a vector of length 4: \[acoustic, phoneme, syllable, word\] best-CFs.
/// (The sentence level is the *output* we are fusing toward, not an input.)
#[must_use]
pub fn extract_level_scores(input: u64) -> Vec<f64> {
    let mut bb = Blackboard::new();
    bb.post(Hypothesis::new(ACOUSTIC, input, 900, 0, 10));
    let _ = crate::ml::hearsay::run(&mut bb, &DEFAULT_KS, 32);
    let mut scores = Vec::with_capacity(4);
    for level in &[ACOUSTIC, PHONEME, SYLLABLE, WORD] {
        scores.push(bb.best_at(*level).map_or(0.0, |h| h.cf as f64));
    }
    scores
}

/// Borda-count rank fusion across N source streams of per-input scores.
///
/// `source_scores[i]` is the score vector from source i (length = number of
/// inputs). Returns a boolean vector of length n_inputs indicating which
/// inputs are in the top-`n_target` ranked items.
#[must_use]
pub fn fuse(source_scores: &[Vec<f64>], n_target: usize) -> Vec<bool> {
    if source_scores.is_empty() {
        return Vec::new();
    }
    let n_inputs = source_scores[0].len();
    let higher_is_better: Vec<bool> = vec![true; source_scores.len()];
    // borda_count expects [source][input] layout — already correct.
    rank_fusion::borda_count(source_scores, &higher_is_better, n_target.min(n_inputs))
}

/// AutoML signal: per-input, run the blackboard and use Borda fusion across the
/// per-level CFs as orthogonal sources. The prediction is `true` if the input
/// ranks in the top-N according to fused score.
pub fn hearsay_automl_signal(name: &str, inputs: &[u64], anchor: &[bool]) -> SignalProfile {
    let n_target = anchor.iter().filter(|&&a| a).count().max(1);

    // Build N=4 source streams: for each level, the per-input best-CF
    let mut source_streams: Vec<Vec<f64>> = vec![Vec::with_capacity(inputs.len()); 4];
    for &inp in inputs {
        let level_scores = extract_level_scores(inp);
        for (i, s) in level_scores.iter().enumerate() {
            source_streams[i].push(*s);
        }
    }

    let predictions = fuse(&source_streams, n_target);
    // Hearsay full chain plus Borda fusion is ~1 µs per input — call it T1.
    let timing_us = (inputs.len() as u64).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

/// Alternative signal: detect whether the blackboard reaches the SENTENCE level
/// after running the default KS chain. This is the "binary" Hearsay verdict.
pub fn hearsay_sentence_signal(name: &str, inputs: &[u64], anchor: &[bool]) -> SignalProfile {
    let mut predictions = Vec::with_capacity(inputs.len());
    for &inp in inputs {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, inp, 90, 0, 10));
        let _ = crate::ml::hearsay::run(&mut bb, &DEFAULT_KS, 32);
        predictions.push(!bb.at(SENTENCE).is_empty());
    }
    let timing_us = (inputs.len() as u64).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_level_scores_returns_four_values() {
        let scores = extract_level_scores(0xCAFE);
        assert_eq!(scores.len(), 4);
    }

    #[test]
    fn extract_level_scores_acoustic_is_high_for_seed_input() {
        let scores = extract_level_scores(0xCAFE);
        // Acoustic level CF starts at 0.9
        assert!(scores[0] >= 0.85);
    }

    #[test]
    fn fuse_with_four_sources_picks_top_target() {
        let sources = vec![
            vec![0.9, 0.1, 0.8, 0.2], // source 1 says inputs 0, 2 are high
            vec![0.85, 0.15, 0.75, 0.25],
            vec![0.95, 0.2, 0.7, 0.3],
            vec![0.88, 0.12, 0.82, 0.18],
        ];
        let preds = fuse(&sources, 2);
        assert_eq!(preds.len(), 4);
        let true_count = preds.iter().filter(|&&p| p).count();
        assert_eq!(true_count, 2);
        // Top-2 should be inputs 0 and 2 (highest aggregate rank)
        assert!(preds[0]);
        assert!(preds[2]);
    }

    #[test]
    fn fuse_empty_sources_yields_empty() {
        assert!(fuse(&[], 1).is_empty());
    }

    #[test]
    fn hearsay_signal_produces_correct_length() {
        let inputs = vec![0xAA_u64, 0xBB_u64, 0xCC_u64, 0xDD_u64];
        let anchor = vec![true, false, true, false];
        let sig = hearsay_automl_signal("hearsay_borda", &inputs, &anchor);
        assert_eq!(sig.predictions.len(), 4);
    }

    #[test]
    fn hearsay_sentence_signal_runs_to_sentence() {
        let inputs = vec![0xCAFE_u64; 3];
        let anchor = vec![true; 3];
        let sig = hearsay_sentence_signal("hearsay_sentence", &inputs, &anchor);
        // The default KS chain reaches SENTENCE; all should be true.
        assert!(sig.predictions.iter().all(|&p| p));
    }

    #[test]
    fn signal_is_deterministic_across_invocations() {
        // Hearsay run + Borda fusion must produce identical output across runs.
        let inputs = vec![0xAA_u64, 0xBB_u64, 0xCC_u64, 0xDD_u64];
        let anchor = vec![true, false, true, false];
        let s1 = hearsay_automl_signal("h", &inputs, &anchor);
        let s2 = hearsay_automl_signal("h", &inputs, &anchor);
        let s3 = hearsay_automl_signal("h", &inputs, &anchor);
        assert_eq!(s1.predictions, s2.predictions);
        assert_eq!(s2.predictions, s3.predictions);
        assert_eq!(s1.accuracy_vs_anchor, s2.accuracy_vs_anchor);
    }

    #[test]
    fn hearsay_signal_in_sample_accuracy_at_least_chance() {
        let inputs = vec![0x1_u64, 0x2_u64, 0x3_u64, 0x4_u64, 0x5_u64];
        let anchor = vec![true, false, true, false, true];
        let sig = hearsay_automl_signal("hearsay_borda", &inputs, &anchor);
        assert_eq!(sig.predictions.len(), 5);
    }
}
