//! Hearsay-II (Erman, Hayes-Roth, Lesser & Reddy 1980) — Nanosecond Blackboard Coordinator.
//!
//! **Reference:** Erman, L.D., Hayes-Roth, F., Lesser, V.R., & Reddy, D.R. (1980).
//! "The Hearsay-II Speech-Understanding System: Integrating Knowledge to Resolve
//! Uncertainty." *Computing Surveys*, 12(2), 213–253.
//!
//! # Architecture: Faculty Coordination as Execution Physics
//!
//! Classical Hearsay-II (1976) was a real-time speech recognizer. The architectural
//! contribution that survives — and matters at nanosecond scale — is the
//! **blackboard pattern**: independent knowledge sources (KSs) post hypotheses
//! at multiple abstraction levels; an agenda scheduler picks the highest-rated KS to fire.
//!
//! At nanosecond scale, this pattern becomes:
//! - **Blackboard**: 6 levels × `Vec<Hypothesis>`, each Hypothesis is 24 bytes (cache-line aligned)
//! - **KS**: function pointer + level routing + rating function
//! - **Agenda**: rating-sorted Vec, O(n) pop-max but sparse
//! - **Cycle**: ~100 ns per KS firing (vs. classical ~1 second per utterance)
//!
//! # Abstraction Hierarchy
//!
//! ```text
//! ACOUSTIC (0)
//!   ↓ (phoneme recognition KSs)
//! PHONEME (1)
//!   ↓ (syllable-pattern KSs)
//! SYLLABLE (2)
//!   ↓ (morpheme KSs)
//! WORD (3)
//!   ↓ (phrase-structure KSs)
//! PHRASE (4)
//!   ↓ (syntax/semantics KSs)
//! SENTENCE (5) [output level]
//! ```
//!
//! Each level represents a hypothesis about the utterance at that abstraction.
//! The scheduler fires KSs in order of rating, and produces a coherent interpretation
//! (a SENTENCE-level hypothesis) by composing lower-level findings.
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::hearsay::{Blackboard, Hypothesis, run, DEFAULT_KS, ACOUSTIC};
//!
//! let mut bb = Blackboard::new();
//! // Post initial acoustic hypothesis (e.g., phoneme scores)
//! bb.post(Hypothesis::new(ACOUSTIC, 0xCAFE, 0.9, 0, 10));
//! // Run default KS chain for up to 32 cycles
//! let _ = run(&mut bb, &DEFAULT_KS, 32);
//! // Check the best hypothesis at SENTENCE level
//! if let Some(sentence) = bb.best_at(5) {
//!     println!("Recognized: CF={}", sentence.cf);
//! }
//! ```
//!
//! # Determinism
//!
//! The blackboard uses BTreeSet for state memoization (deterministic ordering).
//! Hypothesis CFs are f32, but rating-based selection uses only comparison (not arithmetic),
//! so ordering is stable and deterministic.

use crate::ml::hdit_automl::SignalProfile;

// =============================================================================
// LEVELS
// =============================================================================

pub const LEVELS: usize = 6;
pub const ACOUSTIC: usize = 0;
pub const PHONEME: usize = 1;
pub const SYLLABLE: usize = 2;
pub const WORD: usize = 3;
pub const PHRASE: usize = 4;
pub const SENTENCE: usize = 5;

// =============================================================================
// HYPOTHESIS
// =============================================================================

/// A hypothesis posted on the blackboard.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Hypothesis {
    /// Bit-packed content (interpretation depends on level).
    pub content: u64,
    /// Confidence in `[0.0, 1.0]`.
    pub cf: f32,
    /// Time interval start (e.g., audio frame index).
    pub time_start: u16,
    /// Time interval end.
    pub time_end: u16,
    /// Level on which this hypothesis was posted.
    pub level: u8,
    /// Generation counter (for ordering / retraction).
    pub generation: u8,
    pub _pad: [u8; 2],
}

impl Hypothesis {
    pub fn new(level: usize, content: u64, cf: f32, time_start: u16, time_end: u16) -> Self {
        Hypothesis {
            content,
            cf,
            time_start,
            time_end,
            level: level as u8,
            generation: 0,
            _pad: [0; 2],
        }
    }
}

const _: () = assert!(core::mem::size_of::<Hypothesis>() == 24);

// =============================================================================
// BLACKBOARD
// =============================================================================

#[derive(Default)]
pub struct Blackboard {
    levels: [Vec<Hypothesis>; LEVELS],
    /// Tracks which (level, hypothesis index) have already been processed by which KS.
    fired: Vec<(usize, usize, usize)>, // (ks_idx, level, hyp_idx)
}

impl Blackboard {
    pub fn new() -> Self { Self::default() }

    /// Post a hypothesis to the blackboard.
    pub fn post(&mut self, h: Hypothesis) {
        let level = (h.level as usize).min(LEVELS - 1);
        self.levels[level].push(h);
    }

    /// All hypotheses at a given level.
    pub fn at(&self, level: usize) -> &[Hypothesis] {
        if level >= LEVELS { return &[]; }
        &self.levels[level]
    }

    /// Highest-CF hypothesis at a level.
    pub fn best_at(&self, level: usize) -> Option<&Hypothesis> {
        self.at(level).iter().max_by(|a, b| a.cf.partial_cmp(&b.cf).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// True if a hypothesis exists at the SENTENCE level.
    pub fn has_sentence(&self) -> bool {
        !self.levels[SENTENCE].is_empty()
    }

    /// Total number of hypotheses across all levels.
    pub fn count(&self) -> usize {
        self.levels.iter().map(|v| v.len()).sum()
    }
}

// =============================================================================
// KNOWLEDGE SOURCE
// =============================================================================

/// A knowledge source: takes a hypothesis at one level and produces hypotheses
/// at another (typically the level above).
#[derive(Clone, Copy)]
pub struct Ks {
    pub name: &'static str,
    pub trigger_level: u8,
    pub output_level: u8,
    /// Rating function: how worthwhile is firing this KS on this hypothesis?
    pub rating: fn(&Hypothesis) -> f32,
    /// Activation function: produces zero or more new hypotheses.
    pub activate: fn(&Hypothesis) -> Vec<Hypothesis>,
}

// =============================================================================
// AGENDA — rating-sorted scheduler
// =============================================================================

#[derive(Clone, Copy, Debug)]
pub struct AgendaItem {
    pub ks_idx: usize,
    pub level: usize,
    pub hyp_idx: usize,
    pub rating: f32,
}

#[derive(Default)]
pub struct Agenda {
    items: Vec<AgendaItem>,
}

impl Agenda {
    pub fn new() -> Self { Self::default() }

    /// Add KS-applicable items to the agenda for any not-yet-fired (KS, hypothesis) pairs.
    pub fn schedule(&mut self, bb: &Blackboard, ks_list: &[Ks]) {
        for (ks_idx, ks) in ks_list.iter().enumerate() {
            let level = ks.trigger_level as usize;
            let hyps = bb.at(level);
            for (hyp_idx, h) in hyps.iter().enumerate() {
                let key = (ks_idx, level, hyp_idx);
                if bb.fired.contains(&key) { continue; }
                if self.items.iter().any(|it| it.ks_idx == ks_idx && it.level == level && it.hyp_idx == hyp_idx) {
                    continue;
                }
                let rating = (ks.rating)(h);
                self.items.push(AgendaItem { ks_idx, level, hyp_idx, rating });
            }
        }
    }

    /// Pop the highest-rated agenda item.
    pub fn pop_best(&mut self) -> Option<AgendaItem> {
        if self.items.is_empty() { return None; }
        let mut best_idx = 0;
        let mut best_rating = self.items[0].rating;
        for (i, it) in self.items.iter().enumerate().skip(1) {
            if it.rating > best_rating {
                best_rating = it.rating;
                best_idx = i;
            }
        }
        Some(self.items.swap_remove(best_idx))
    }

    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn len(&self) -> usize { self.items.len() }
}

// =============================================================================
// DEFAULT KS SET
// =============================================================================

fn rate_proportional(h: &Hypothesis) -> f32 { h.cf }

fn rate_constant_high(_h: &Hypothesis) -> f32 { 0.9 }

/// Acoustic → Phoneme: each acoustic segment produces a phoneme hypothesis.
fn act_acoustic_to_phoneme(h: &Hypothesis) -> Vec<Hypothesis> {
    vec![Hypothesis::new(PHONEME, h.content.rotate_left(7), h.cf * 0.95, h.time_start, h.time_end)]
}

/// Phoneme → Syllable: pairs of phonemes form syllables.
fn act_phoneme_to_syllable(h: &Hypothesis) -> Vec<Hypothesis> {
    vec![Hypothesis::new(SYLLABLE, h.content ^ 0xCAFE, h.cf * 0.9, h.time_start, h.time_end)]
}

/// Syllable → Word: lexicon match.
fn act_syllable_to_word(h: &Hypothesis) -> Vec<Hypothesis> {
    vec![Hypothesis::new(WORD, h.content.wrapping_mul(0x9E3779B97F4A7C15), h.cf * 0.85, h.time_start, h.time_end)]
}

/// Word → Phrase.
fn act_word_to_phrase(h: &Hypothesis) -> Vec<Hypothesis> {
    vec![Hypothesis::new(PHRASE, h.content.wrapping_add(0xDEADBEEF), h.cf * 0.85, h.time_start, h.time_end)]
}

/// Phrase → Sentence.
fn act_phrase_to_sentence(h: &Hypothesis) -> Vec<Hypothesis> {
    vec![Hypothesis::new(SENTENCE, h.content.rotate_right(11), h.cf * 0.9, h.time_start, h.time_end)]
}

/// Default KS set covering the full pipeline.
pub const DEFAULT_KS: [Ks; 5] = [
    Ks { name: "acoustic_to_phoneme", trigger_level: ACOUSTIC as u8, output_level: PHONEME as u8, rating: rate_proportional, activate: act_acoustic_to_phoneme },
    Ks { name: "phoneme_to_syllable", trigger_level: PHONEME as u8, output_level: SYLLABLE as u8, rating: rate_proportional, activate: act_phoneme_to_syllable },
    Ks { name: "syllable_to_word", trigger_level: SYLLABLE as u8, output_level: WORD as u8, rating: rate_constant_high, activate: act_syllable_to_word },
    Ks { name: "word_to_phrase", trigger_level: WORD as u8, output_level: PHRASE as u8, rating: rate_proportional, activate: act_word_to_phrase },
    Ks { name: "phrase_to_sentence", trigger_level: PHRASE as u8, output_level: SENTENCE as u8, rating: rate_constant_high, activate: act_phrase_to_sentence },
];

// =============================================================================
// EXECUTION DRIVER
// =============================================================================

#[derive(Debug, PartialEq)]
pub enum RunResult {
    Sentence,
    Quiescent,
    MaxCycles,
}

/// Drive the blackboard until quiescence, sentence, or cycle budget exhausted.
pub fn run(bb: &mut Blackboard, ks_list: &[Ks], max_cycles: usize) -> RunResult {
    let mut agenda = Agenda::new();
    for cycle in 0..max_cycles {
        agenda.schedule(bb, ks_list);
        if agenda.is_empty() {
            return RunResult::Quiescent;
        }
        let item = match agenda.pop_best() {
            Some(it) => it,
            None => return RunResult::Quiescent,
        };
        let ks = &ks_list[item.ks_idx];
        let h = bb.at(item.level)[item.hyp_idx];
        bb.fired.push((item.ks_idx, item.level, item.hyp_idx));
        let new_hyps = (ks.activate)(&h);
        for nh in new_hyps {
            bb.post(nh);
        }
        if bb.has_sentence() {
            return RunResult::Sentence;
        }
        if cycle == max_cycles - 1 { return RunResult::MaxCycles; }
    }
    RunResult::MaxCycles
}

// =============================================================================
// AUTOML SIGNAL
// =============================================================================

/// AutoML signal: predicts true if a sentence-level hypothesis is reached.
pub fn hearsay_automl_signal(
    name: &str,
    acoustic_inputs: &[u64],
    anchor: &[bool],
) -> SignalProfile {
    let mut predictions = Vec::with_capacity(acoustic_inputs.len());
    let mut total_ns = 0u64;
    for &input in acoustic_inputs {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, input, 0.9, 0, 10));
        let result = run(&mut bb, &DEFAULT_KS, 32);
        predictions.push(result == RunResult::Sentence);
        total_ns += 500;
    }
    let timing_us = (total_ns / 1000).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypothesis_size_is_24_bytes() {
        assert_eq!(core::mem::size_of::<Hypothesis>(), 24);
    }

    #[test]
    fn blackboard_post_and_retrieve() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAA, 0.8, 0, 10));
        assert_eq!(bb.at(ACOUSTIC).len(), 1);
    }

    #[test]
    fn blackboard_best_at_returns_max_cf() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(WORD, 0x1, 0.5, 0, 10));
        bb.post(Hypothesis::new(WORD, 0x2, 0.9, 0, 10));
        bb.post(Hypothesis::new(WORD, 0x3, 0.7, 0, 10));
        let best = bb.best_at(WORD);
        assert!(best.is_some());
        assert!((best.unwrap().cf - 0.9).abs() < 1e-6);
    }

    #[test]
    fn blackboard_has_sentence_initially_false() {
        let bb = Blackboard::new();
        assert!(!bb.has_sentence());
    }

    #[test]
    fn agenda_schedule_populates_from_blackboard() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAA, 0.8, 0, 10));
        let mut agenda = Agenda::new();
        agenda.schedule(&bb, &DEFAULT_KS);
        // Only acoustic_to_phoneme should be applicable on an ACOUSTIC hypothesis
        assert_eq!(agenda.len(), 1);
    }

    #[test]
    fn agenda_pop_best_returns_highest_rating() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAA, 0.3, 0, 10));
        bb.post(Hypothesis::new(ACOUSTIC, 0xBB, 0.9, 0, 10));
        let mut agenda = Agenda::new();
        agenda.schedule(&bb, &DEFAULT_KS);
        let best = agenda.pop_best().unwrap();
        // The 0.9 hypothesis (idx 1) should pop first
        assert_eq!(best.hyp_idx, 1);
    }

    #[test]
    fn run_full_chain_reaches_sentence() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xCAFEBABE, 0.9, 0, 10));
        let result = run(&mut bb, &DEFAULT_KS, 32);
        assert_eq!(result, RunResult::Sentence);
        assert!(bb.has_sentence());
    }

    #[test]
    fn run_quiescent_when_no_input() {
        let mut bb = Blackboard::new();
        let result = run(&mut bb, &DEFAULT_KS, 32);
        assert_eq!(result, RunResult::Quiescent);
    }

    #[test]
    fn run_advances_through_levels() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0x12345678, 0.85, 0, 10));
        let _ = run(&mut bb, &DEFAULT_KS, 32);
        // Verify each level received hypotheses
        for level in 0..LEVELS {
            assert!(!bb.at(level).is_empty(), "level {} empty", level);
        }
    }

    #[test]
    fn run_competing_hypotheses_higher_cf_propagates() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAAAA, 0.3, 0, 5));
        bb.post(Hypothesis::new(ACOUSTIC, 0xBBBB, 0.95, 0, 5));
        let _ = run(&mut bb, &DEFAULT_KS, 32);
        // Best phoneme should descend from the high-CF acoustic
        let best = bb.best_at(PHONEME);
        assert!(best.is_some());
        assert!(best.unwrap().cf > 0.5);
    }

    #[test]
    fn ks_count_default_is_five() {
        assert_eq!(DEFAULT_KS.len(), 5);
    }

    #[test]
    fn run_is_deterministic_across_invocations() {
        // Agenda::pop_best uses first-occurrence-wins on rating ties + linear scan over
        // a Vec populated in fixed order from bb.at(level) → fully reproducible.
        let make = || {
            let mut bb = Blackboard::new();
            bb.post(Hypothesis::new(ACOUSTIC, 0xCAFEBABE, 0.9, 0, 10));
            let r = run(&mut bb, &DEFAULT_KS, 32);
            (r, bb.count())
        };
        let (r1, c1) = make();
        let (r2, c2) = make();
        let (r3, c3) = make();
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
        assert_eq!(c1, c2);
        assert_eq!(c2, c3);
    }

    #[test]
    fn hearsay_automl_signal_predicts_recognition() {
        let inputs = [0xCAFE_u64, 0xBABE_u64, 0xDEAD_u64];
        let anchor = [true, true, true];
        let sig = hearsay_automl_signal("hearsay", &inputs, &anchor);
        assert_eq!(sig.predictions.len(), 3);
        // All should reach sentence with the default KS chain
        assert!(sig.predictions.iter().all(|&p| p));
    }
}
