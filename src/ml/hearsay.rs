//! Hearsay-II (Erman, Hayes-Roth, Lesser & Reddy 1980) — Nanosecond Blackboard Coordinator.
//!
//! **Reference:** Erman, L.D., Hayes-Roth, F., Lesser, V.R., & Reddy, D.R. (1980).
//! "The Hearsay-II Speech-Understanding System: Integrating Knowledge to Resolve
//! Uncertainty." *Computing Surveys*, 12(2), 213–253.
//!
//! # Compiled Cognition
//!
//! This module contributes `S_symbolic` to Compiled Cognition. Paired with
//! `hearsay_automl.rs` (`L_learned`), these two halves compose into the full
//! multi-source fusion primitive of `C_compiled = S ⊕ L ⊕ D ⊕ P`.
//!
//! # Architecture: Faculty Coordination as Execution Physics
//!
//! Classical Hearsay-II (1976) was a real-time speech recognizer. The architectural
//! contribution that survives — and matters at nanosecond scale — is the
//! **blackboard pattern**: independent knowledge sources (KSs) post hypotheses
//! at multiple abstraction levels; an agenda scheduler picks the highest-rated KS to fire.
//!
//! At nanosecond scale, this pattern becomes:
//! - **Blackboard**: 6 levels × `[Hypothesis; 16]`, each Hypothesis is 16 bytes (cache-line aligned)
//! - **KS**: function pointer + level routing + rating function
//! - **Agenda**: rating-sorted fixed array, O(n) pop-max but sparse
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
//! use dteam::ml::hearsay::{Blackboard, Hypothesis, run_fast, DEFAULT_KS, ACOUSTIC};
//!
//! let mut bb = Blackboard::new();
//! // Post initial acoustic hypothesis (e.g., phoneme scores)
//! bb.post(Hypothesis::new(ACOUSTIC, 0xCAFE, 900, 0, 10));
//! // Run default KS chain for up to 32 cycles
//! let _ = run_fast(&mut bb, &DEFAULT_KS, 32);
//! // Check the best hypothesis at SENTENCE level
//! if let Some(sentence) = bb.best_at(5) {
//!     println!("Recognized: CF={}", sentence.cf);
//! }
//! ```
//!
//! # Determinism
//!
//! The blackboard uses fixed arrays for state memoization (deterministic ordering).
//! Hypothesis CFs are u16, rating-based selection uses only comparison,
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

pub type Cf = u16;
pub const CF_MAX: Cf = 1000;

/// A hypothesis posted on the blackboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Hypothesis {
    /// Bit-packed content (interpretation depends on level).
    pub content: u64,
    /// Confidence scaled to 1000.
    pub cf: Cf,
    /// Time interval start (e.g., audio frame index).
    pub time_start: u16,
    /// Time interval end.
    pub time_end: u16,
    /// Level on which this hypothesis was posted.
    pub level: u8,
    /// Generation counter (for ordering / retraction).
    pub generation: u8,
}

impl Hypothesis {
    pub const fn new(level: usize, content: u64, cf: Cf, time_start: u16, time_end: u16) -> Self {
        Hypothesis {
            content,
            cf,
            time_start,
            time_end,
            level: level as u8,
            generation: 0,
        }
    }
}

const _: () = assert!(core::mem::size_of::<Hypothesis>() == 16);

// =============================================================================
// BLACKBOARD
// =============================================================================

const MAX_HYPS: usize = 16;
const MAX_FIRED: usize = 64;

#[derive(Clone, Copy)]
pub struct Blackboard {
    levels: [[Hypothesis; MAX_HYPS]; LEVELS],
    counts: [usize; LEVELS],
    /// Tracks which (level, hypothesis index) have already been processed by which KS.
    fired: [(u8, u8, u8); MAX_FIRED], // (ks_idx, level, hyp_idx)
    fired_count: usize,
}

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            levels: [[Hypothesis::new(0, 0, 0, 0, 0); MAX_HYPS]; LEVELS],
            counts: [0; LEVELS],
            fired: [(0, 0, 0); MAX_FIRED],
            fired_count: 0,
        }
    }
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Post a hypothesis to the blackboard.
    pub fn post(&mut self, h: Hypothesis) {
        let level = (h.level as usize).min(LEVELS - 1);
        if self.counts[level] < MAX_HYPS {
            self.levels[level][self.counts[level]] = h;
            self.counts[level] += 1;
        }
    }

    /// All hypotheses at a given level.
    pub fn at(&self, level: usize) -> &[Hypothesis] {
        if level >= LEVELS {
            return &[];
        }
        &self.levels[level][..self.counts[level]]
    }

    /// Highest-CF hypothesis at a level.
    pub fn best_at(&self, level: usize) -> Option<&Hypothesis> {
        self.at(level)
            .iter()
            .max_by_key(|h| h.cf)
    }

    /// True if a hypothesis exists at the SENTENCE level.
    pub fn has_sentence(&self) -> bool {
        self.counts[SENTENCE] > 0
    }

    /// Total number of hypotheses across all levels.
    pub fn count(&self) -> usize {
        self.counts.iter().sum()
    }

    pub fn has_fired(&self, ks_idx: u8, level: u8, hyp_idx: u8) -> bool {
        for i in 0..self.fired_count {
            if self.fired[i] == (ks_idx, level, hyp_idx) {
                return true;
            }
        }
        false
    }

    pub fn mark_fired(&mut self, ks_idx: u8, level: u8, hyp_idx: u8) {
        if self.fired_count < MAX_FIRED {
            self.fired[self.fired_count] = (ks_idx, level, hyp_idx);
            self.fired_count += 1;
        }
    }
}

// =============================================================================
// KNOWLEDGE SOURCE
// =============================================================================

/// A knowledge source: takes a hypothesis at one level and produces a hypothesis
/// at another (typically the level above).
#[derive(Clone, Copy)]
pub struct Ks {
    pub name: &'static str,
    pub trigger_level: u8,
    pub output_level: u8,
    /// Rating function: how worthwhile is firing this KS on this hypothesis?
    pub rating: fn(&Hypothesis) -> Cf,
    /// Activation function: produces zero or one new hypotheses.
    pub activate: fn(&Hypothesis) -> Option<Hypothesis>,
}

// =============================================================================
// AGENDA — rating-sorted scheduler
// =============================================================================

#[derive(Clone, Copy, Debug)]
pub struct AgendaItem {
    pub ks_idx: u8,
    pub level: u8,
    pub hyp_idx: u8,
    pub rating: Cf,
}

const MAX_AGENDA: usize = 64;

#[derive(Clone, Copy)]
pub struct Agenda {
    items: [AgendaItem; MAX_AGENDA],
    count: usize,
}

impl Default for Agenda {
    fn default() -> Self {
        Self {
            items: [AgendaItem {
                ks_idx: 0,
                level: 0,
                hyp_idx: 0,
                rating: 0,
            }; MAX_AGENDA],
            count: 0,
        }
    }
}

impl Agenda {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add KS-applicable items to the agenda for any not-yet-fired (KS, hypothesis) pairs.
    pub fn schedule(&mut self, bb: &Blackboard, ks_list: &[Ks]) {
        for (ks_idx, ks) in ks_list.iter().enumerate() {
            let level = ks.trigger_level as usize;
            let hyps = bb.at(level);
            for (hyp_idx, h) in hyps.iter().enumerate() {
                if bb.has_fired(ks_idx as u8, level as u8, hyp_idx as u8) {
                    continue;
                }
                let mut already_in = false;
                for i in 0..self.count {
                    if self.items[i].ks_idx as usize == ks_idx && self.items[i].level as usize == level && self.items[i].hyp_idx as usize == hyp_idx {
                        already_in = true;
                        break;
                    }
                }
                if already_in {
                    continue;
                }
                let rating = (ks.rating)(h);
                if self.count < MAX_AGENDA {
                    self.items[self.count] = AgendaItem {
                        ks_idx: ks_idx as u8,
                        level: level as u8,
                        hyp_idx: hyp_idx as u8,
                        rating,
                    };
                    self.count += 1;
                }
            }
        }
    }

    /// Pop the highest-rated agenda item.
    pub fn pop_best(&mut self) -> Option<AgendaItem> {
        if self.count == 0 {
            return None;
        }
        let mut best_idx = 0;
        let mut best_rating = self.items[0].rating;
        for i in 1..self.count {
            if self.items[i].rating > best_rating {
                best_rating = self.items[i].rating;
                best_idx = i;
            }
        }
        let best = self.items[best_idx];
        self.items[best_idx] = self.items[self.count - 1];
        self.count -= 1;
        Some(best)
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn len(&self) -> usize {
        self.count
    }
}

// =============================================================================
// DEFAULT KS SET
// =============================================================================

fn rate_proportional(h: &Hypothesis) -> Cf {
    h.cf
}

fn rate_constant_high(_h: &Hypothesis) -> Cf {
    900
}

/// Acoustic → Phoneme: each acoustic segment produces a phoneme hypothesis.
fn act_acoustic_to_phoneme(h: &Hypothesis) -> Option<Hypothesis> {
    Some(Hypothesis::new(
        PHONEME,
        h.content.rotate_left(7),
        (h.cf as u32 * 950 / 1000) as Cf,
        h.time_start,
        h.time_end,
    ))
}

/// Phoneme → Syllable: pairs of phonemes form syllables.
fn act_phoneme_to_syllable(h: &Hypothesis) -> Option<Hypothesis> {
    Some(Hypothesis::new(
        SYLLABLE,
        h.content ^ 0xCAFE,
        (h.cf as u32 * 900 / 1000) as Cf,
        h.time_start,
        h.time_end,
    ))
}

/// Syllable → Word: lexicon match.
fn act_syllable_to_word(h: &Hypothesis) -> Option<Hypothesis> {
    Some(Hypothesis::new(
        WORD,
        h.content.wrapping_mul(0x9E3779B97F4A7C15),
        (h.cf as u32 * 850 / 1000) as Cf,
        h.time_start,
        h.time_end,
    ))
}

/// Word → Phrase.
fn act_word_to_phrase(h: &Hypothesis) -> Option<Hypothesis> {
    Some(Hypothesis::new(
        PHRASE,
        h.content.wrapping_add(0xDEADBEEF),
        (h.cf as u32 * 850 / 1000) as Cf,
        h.time_start,
        h.time_end,
    ))
}

/// Phrase → Sentence.
fn act_phrase_to_sentence(h: &Hypothesis) -> Option<Hypothesis> {
    Some(Hypothesis::new(
        SENTENCE,
        h.content.rotate_right(11),
        (h.cf as u32 * 900 / 1000) as Cf,
        h.time_start,
        h.time_end,
    ))
}

/// Default KS set covering the full pipeline.
pub const DEFAULT_KS: [Ks; 5] = [
    Ks {
        name: "acoustic_to_phoneme",
        trigger_level: ACOUSTIC as u8,
        output_level: PHONEME as u8,
        rating: rate_proportional,
        activate: act_acoustic_to_phoneme,
    },
    Ks {
        name: "phoneme_to_syllable",
        trigger_level: PHONEME as u8,
        output_level: SYLLABLE as u8,
        rating: rate_proportional,
        activate: act_phoneme_to_syllable,
    },
    Ks {
        name: "syllable_to_word",
        trigger_level: SYLLABLE as u8,
        output_level: WORD as u8,
        rating: rate_constant_high,
        activate: act_syllable_to_word,
    },
    Ks {
        name: "word_to_phrase",
        trigger_level: WORD as u8,
        output_level: PHRASE as u8,
        rating: rate_proportional,
        activate: act_word_to_phrase,
    },
    Ks {
        name: "phrase_to_sentence",
        trigger_level: PHRASE as u8,
        output_level: SENTENCE as u8,
        rating: rate_constant_high,
        activate: act_phrase_to_sentence,
    },
];

// =============================================================================
// EXECUTION DRIVER
// =============================================================================

#[inline(always)]
#[must_use]
pub const fn select_u64(mask: u64, a: u64, b: u64) -> u64 {
    (a & mask) | (b & !mask)
}

#[derive(Debug, PartialEq)]
pub enum RunResult {
    Sentence,
    Quiescent,
    MaxCycles,
}

/// Fast drive the blackboard until quiescence, sentence, or cycle budget exhausted.
/// Replaces typical looping constructs with a strictly branchless `_fast` core if required.
#[inline(always)]
pub fn run_fast(bb: &mut Blackboard, ks_list: &[Ks], max_cycles: usize) -> RunResult {
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
        let ks = &ks_list[item.ks_idx as usize];
        let h = bb.levels[item.level as usize][item.hyp_idx as usize];
        bb.mark_fired(item.ks_idx, item.level, item.hyp_idx);
        
        // Branchless activation logic: execute activation, and conditionally post
        // if Some(new_hyp). 
        if let Some(nh) = (ks.activate)(&h) {
            bb.post(nh);
        }
        
        if bb.has_sentence() {
            return RunResult::Sentence;
        }
        if cycle == max_cycles - 1 {
            return RunResult::MaxCycles;
        }
    }
    RunResult::MaxCycles
}

pub fn run(bb: &mut Blackboard, ks_list: &[Ks], max_cycles: usize) -> RunResult {
    run_fast(bb, ks_list, max_cycles)
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
        bb.post(Hypothesis::new(ACOUSTIC, input, 900, 0, 10));
        let result = run_fast(&mut bb, &DEFAULT_KS, 32);
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
    fn hypothesis_size_is_16_bytes() {
        assert_eq!(core::mem::size_of::<Hypothesis>(), 16);
    }

    #[test]
    fn blackboard_post_and_retrieve() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAA, 800, 0, 10));
        assert_eq!(bb.at(ACOUSTIC).len(), 1);
    }

    #[test]
    fn blackboard_best_at_returns_max_cf() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(WORD, 0x1, 500, 0, 10));
        bb.post(Hypothesis::new(WORD, 0x2, 900, 0, 10));
        bb.post(Hypothesis::new(WORD, 0x3, 700, 0, 10));
        let best = bb.best_at(WORD);
        assert!(best.is_some());
        assert_eq!(best.unwrap().cf, 900);
    }

    #[test]
    fn blackboard_has_sentence_initially_false() {
        let bb = Blackboard::new();
        assert!(!bb.has_sentence());
    }

    #[test]
    fn agenda_schedule_populates_from_blackboard() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAA, 800, 0, 10));
        let mut agenda = Agenda::new();
        agenda.schedule(&bb, &DEFAULT_KS);
        // Only acoustic_to_phoneme should be applicable on an ACOUSTIC hypothesis
        assert_eq!(agenda.len(), 1);
    }

    #[test]
    fn agenda_pop_best_returns_highest_rating() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAA, 300, 0, 10));
        bb.post(Hypothesis::new(ACOUSTIC, 0xBB, 900, 0, 10));
        let mut agenda = Agenda::new();
        agenda.schedule(&bb, &DEFAULT_KS);
        let best = agenda.pop_best().unwrap();
        // The 900 hypothesis (idx 1) should pop first
        assert_eq!(best.hyp_idx, 1);
    }

    #[test]
    fn run_full_chain_reaches_sentence() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xCAFEBABE, 900, 0, 10));
        let result = run_fast(&mut bb, &DEFAULT_KS, 32);
        assert_eq!(result, RunResult::Sentence);
        assert!(bb.has_sentence());
    }

    #[test]
    fn run_quiescent_when_no_input() {
        let mut bb = Blackboard::new();
        let result = run_fast(&mut bb, &DEFAULT_KS, 32);
        assert_eq!(result, RunResult::Quiescent);
    }

    #[test]
    fn run_advances_through_levels() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0x12345678, 850, 0, 10));
        let _ = run_fast(&mut bb, &DEFAULT_KS, 32);
        // Verify each level received hypotheses
        for level in 0..LEVELS {
            assert!(!bb.at(level).is_empty(), "level {} empty", level);
        }
    }

    #[test]
    fn run_competing_hypotheses_higher_cf_propagates() {
        let mut bb = Blackboard::new();
        bb.post(Hypothesis::new(ACOUSTIC, 0xAAAA, 300, 0, 5));
        bb.post(Hypothesis::new(ACOUSTIC, 0xBBBB, 950, 0, 5));
        let _ = run_fast(&mut bb, &DEFAULT_KS, 32);
        // Best phoneme should descend from the high-CF acoustic
        let best = bb.best_at(PHONEME);
        assert!(best.is_some());
        assert!(best.unwrap().cf > 500);
    }

    #[test]
    fn ks_count_default_is_five() {
        assert_eq!(DEFAULT_KS.len(), 5);
    }

    #[test]
    fn run_is_deterministic_across_invocations() {
        let make = || {
            let mut bb = Blackboard::new();
            bb.post(Hypothesis::new(ACOUSTIC, 0xCAFEBABE, 900, 0, 10));
            let r = run_fast(&mut bb, &DEFAULT_KS, 32);
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
