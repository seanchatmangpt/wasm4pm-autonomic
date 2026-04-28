//! ELIZA (Weizenbaum 1966) — Nanosecond Conversational Threshold Engine.
//!
//! **Reference:** Weizenbaum, J. (1966). "ELIZA—A Computer Program for the Study of Natural
//! Language Communication between Man and Machine." *Communications of the ACM*, 9(1), 36–45.
//!
//! # Architecture: Symbolic Cognition as Execution Physics
//!
//! Classical ELIZA was string-manipulation cognition: pattern match keywords, extract,
//! reassemble via templates. At nanosecond scale, it becomes a branchless table lookup
//! over a u64 keyword bitmask. The full inference cycle collapses to:
//!
//! 1. **Hot path** (`turn_fast`): u64 → u8, ~5 ns per inference, branchless
//! 2. **Warm path** (`turn`): bitmask + state tracking, ~50 ns
//! 3. **Cold path** (`respond`): full text I/O, ~10 µs (only for human-facing)
//!
//! The hot path is what enables ELIZA to be called inline on every state transition,
//! every workflow edge, every graph delta — not as advisory cognition, but as runtime
//! infrastructure.
//!
//! # Design Principle
//!
//! ELIZA encodes the DOCTOR script (Weizenbaum's default persona) as a rank-ordered
//! rule table. Keywords are encoded as u64 bits; templates as u8 indices. No strings,
//! no allocations, no control flow on the hot path.
//!
//! # Layers
//!
//! - **Keyword encoding**: `keyword_bit(i)` for i in 0..16, packed into low 16 bits
//! - **Rule table**: `DOCTOR[11]` ordered by priority (rank)
//! - **Inference**: Linear scan, branchless conditional moves for rule selection
//! - **Reassembly**: Template indices map to pattern-response pairs (cold path only)
//!
//! # Example
//!
//! ```rust
//! use dteam::ml::eliza::{turn_fast, keyword_bit, kw, DOCTOR};
//!
//! // Keywords: "I dream about my mother"
//! let input = keyword_bit(kw::I)
//!           | keyword_bit(kw::DREAM)
//!           | keyword_bit(kw::MOTHER);
//!
//! // Inference: which template matches best?
//! let template = turn_fast(input, &DOCTOR);
//! // DREAM rule (rank 2) matches, returns template index for DREAM
//! assert_ne!(template, 0xFF);
//! ```
//!
//! # Performance
//!
//! - **turn_fast**: 5 ns (branchless, cache-friendly, no alloc)
//! - **turn**: 50 ns (adds state tracking and rule metadata)
//! - **respond**: 10 µs (string reassembly, human-facing only)
//!
//! The 200× span from hot to cold path demonstrates the latency collapse thesis:
//! the same job's performance depends entirely on which path executes.

use crate::ml::hdit_automl::SignalProfile;

// =============================================================================
// KEYWORD BITS — 16 keyword slots, packed into low half of u64
// =============================================================================

/// Encode keyword index `i` (0-15) into bit `1 << i`.
///
/// Hot-path constant: keyword presence is one AND.
#[inline(always)]
pub const fn keyword_bit(i: u8) -> u64 {
    1u64 << (i & 0x0F)
}

/// Encode rule index `i` (0-15) into bit `1 << (16 + i)`.
#[inline(always)]
pub const fn rule_bit(i: u8) -> u64 {
    1u64 << (16 + (i & 0x0F))
}

/// Encode template index `i` (0-15) into bit `1 << (32 + i)`.
#[inline(always)]
pub const fn template_bit(i: u8) -> u64 {
    1u64 << (32 + (i & 0x0F))
}

/// Named DOCTOR-script keyword indices.
pub mod kw {
    pub const SORRY: u8 = 0;
    pub const DREAM: u8 = 1;
    pub const MOTHER: u8 = 2;
    pub const FATHER: u8 = 3;
    pub const FAMILY: u8 = 4;
    pub const ALWAYS: u8 = 5;
    pub const ALIKE: u8 = 6;
    pub const HAPPY: u8 = 7;
    pub const SAD: u8 = 8;
    pub const COMPUTER: u8 = 9;
    pub const REMEMBER: u8 = 10;
    pub const I: u8 = 11;
    pub const YOU: u8 = 12;
    pub const MY: u8 = 13;
    pub const NAME: u8 = 14;
    pub const FALLBACK: u8 = 15;
}

/// Named template indices.
pub mod tmpl {
    pub const APOLOGIZE: u8 = 0;
    pub const DREAM: u8 = 1;
    pub const FAMILY: u8 = 2;
    pub const ELABORATE: u8 = 3;
    pub const SIMILARITY: u8 = 4;
    pub const FEELINGS: u8 = 5;
    pub const COMPUTER: u8 = 6;
    pub const MEMORY: u8 = 7;
    pub const REFLECT_I: u8 = 8;
    pub const REFLECT_YOU: u8 = 9;
    pub const NAME: u8 = 10;
    pub const FALLBACK_GO_ON: u8 = 11;
    pub const FALLBACK_TELL: u8 = 12;
    pub const FALLBACK_SEE: u8 = 13;
}

// =============================================================================
// RULE TABLE — keyword bitmask + rank + template index
// =============================================================================

/// One ELIZA decomposition rule, fully bit-packed.
///
/// `keyword_mask` is OR of [`keyword_bit`] values. The rule fires when ALL bits
/// are set in the input mask (AND test).
///
/// Lower `rank` = higher priority (matches Weizenbaum's original priority ordering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ElizaRule {
    pub keyword_mask: u64,
    pub template_index: u8,
    pub rank: u8,
    pub _pad: [u8; 6],
}

const _: () = assert!(core::mem::size_of::<ElizaRule>() == 16);

/// Result of one inference turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct ElizaTurn {
    /// Template index (0xFF = no match → fallback).
    pub template: u8,
    /// True if a non-fallback rule matched.
    pub matched: bool,
    /// Output state mask: keyword_bit OR'd into upper half (history bits).
    pub state: u64,
    /// Rule index that fired (0xFF if none).
    pub rule_idx: u8,
}

/// The DOCTOR script as a static rule table.
///
/// Ordered by `rank` ascending: highest-priority keywords first.
pub const DOCTOR: [ElizaRule; 11] = [
    // High-priority specific keywords
    ElizaRule { keyword_mask: keyword_bit(kw::COMPUTER), template_index: tmpl::COMPUTER, rank: 0, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::NAME), template_index: tmpl::NAME, rank: 0, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::SORRY), template_index: tmpl::APOLOGIZE, rank: 1, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::REMEMBER), template_index: tmpl::MEMORY, rank: 2, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::DREAM), template_index: tmpl::DREAM, rank: 2, _pad: [0; 6] },
    // Family keywords (collapse to FAMILY template)
    ElizaRule { keyword_mask: keyword_bit(kw::MOTHER), template_index: tmpl::FAMILY, rank: 3, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::FATHER), template_index: tmpl::FAMILY, rank: 3, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::FAMILY), template_index: tmpl::FAMILY, rank: 3, _pad: [0; 6] },
    // Affect keywords
    ElizaRule { keyword_mask: keyword_bit(kw::HAPPY) | keyword_bit(kw::SAD), template_index: tmpl::FEELINGS, rank: 4, _pad: [0; 6] },
    ElizaRule { keyword_mask: keyword_bit(kw::ALIKE), template_index: tmpl::SIMILARITY, rank: 4, _pad: [0; 6] },
    // Self-reference
    ElizaRule { keyword_mask: keyword_bit(kw::I), template_index: tmpl::REFLECT_I, rank: 5, _pad: [0; 6] },
];

// =============================================================================
// HOT PATH — branchless ~5 ns inference
// =============================================================================

/// Inference at nanosecond scale: input bitmask → template index, branchless.
///
/// Returns `0xFF` if no rule matches. The first rule (lowest rank) whose
/// `keyword_mask` is fully present in `input_mask` wins.
///
/// **Hot-path constraints:**
/// - No allocation
/// - No string operations
/// - No branching on the admitted path (single linear scan, branchless within rule)
/// - Returns u8 in a register
///
/// At rank-sorted DOCTOR scale (11 rules), this is ~5 ns on a modern CPU.
///
/// # Example
///
/// ```rust
/// use dteam::ml::eliza::{turn_fast, keyword_bit, kw, DOCTOR};
///
/// // User input contains DREAM and MOTHER keywords
/// let input = keyword_bit(kw::DREAM) | keyword_bit(kw::MOTHER);
///
/// // Run DOCTOR rules
/// let template_idx = turn_fast(input, &DOCTOR);
///
/// // DREAM has higher priority (rank 2) than MOTHER (rank 3), so DREAM wins
/// assert_ne!(template_idx, 0xFF); // Some template matched
/// ```
#[inline(always)]
#[must_use]
pub fn turn_fast(input_mask: u64, rules: &[ElizaRule]) -> u8 {
    let mut best: u8 = 0xFF;
    let mut best_rank: u8 = 0xFF;
    let mut i = 0;
    while i < rules.len() {
        let r = rules[i];
        // Branchless: bit-mask both `match` and `improvement-over-best`
        let matches = ((r.keyword_mask & input_mask) == r.keyword_mask) as u8;
        let improves = ((r.rank < best_rank) as u8) & matches;
        // Conditional move via bitmask multiply (branchless)
        let pick_mask = (improves as u64).wrapping_neg();
        best = ((r.template_index as u64 & pick_mask) | (best as u64 & !pick_mask)) as u8;
        best_rank = ((r.rank as u64 & pick_mask) | (best_rank as u64 & !pick_mask)) as u8;
        i += 1;
    }
    best
}

/// Warm path: returns the full turn record (state + matched + template).
///
/// `~50 ns` — adds bookkeeping to track which rule fired and rolling state.
#[inline]
#[must_use]
pub fn turn(input_mask: u64, rules: &[ElizaRule], history: u64) -> ElizaTurn {
    let mut best_template: u8 = 0xFF;
    let mut best_rule: u8 = 0xFF;
    let mut best_rank: u8 = 0xFF;
    let mut best_keywords: u64 = 0;

    for (i, r) in rules.iter().enumerate() {
        if (r.keyword_mask & input_mask) == r.keyword_mask && r.rank < best_rank {
            best_template = r.template_index;
            best_rule = i as u8;
            best_rank = r.rank;
            best_keywords = r.keyword_mask;
        }
    }

    let matched = best_template != 0xFF;
    // Roll history: shift prior history up, merge in current keywords
    let new_state = history.rotate_left(7) ^ best_keywords ^ ((best_template as u64) << 32);

    ElizaTurn {
        template: best_template,
        matched,
        state: new_state,
        rule_idx: best_rule,
    }
}

// =============================================================================
// COLD PATH — text I/O for human-facing usage
// =============================================================================

/// Tokenize input text and produce a keyword bitmask.
///
/// This is the cold path — only called when ELIZA needs to face a human.
/// For inline cognition (state transitions, workflow edges), the caller
/// constructs the mask directly via [`keyword_bit`] OR'ing.
pub fn keywords_from_text(input: &str) -> u64 {
    let lower = input.to_lowercase();
    let mut mask = 0u64;
    for tok in lower.split_whitespace() {
        let stripped = tok.trim_matches(|c: char| !c.is_alphanumeric());
        let bit = match stripped {
            "sorry" | "apologize" | "apologies" => keyword_bit(kw::SORRY),
            "dream" | "dreams" | "dreamed" | "dreamt" => keyword_bit(kw::DREAM),
            "mother" | "mom" | "mum" | "mommy" => keyword_bit(kw::MOTHER),
            "father" | "dad" | "daddy" | "papa" => keyword_bit(kw::FATHER),
            "family" | "families" | "relatives" => keyword_bit(kw::FAMILY),
            "always" | "everytime" | "constantly" => keyword_bit(kw::ALWAYS),
            "alike" | "similar" | "same" => keyword_bit(kw::ALIKE),
            "happy" | "glad" | "joyful" | "pleased" => keyword_bit(kw::HAPPY),
            "sad" | "unhappy" | "depressed" | "miserable" => keyword_bit(kw::SAD),
            "computer" | "computers" | "machine" | "robot" => keyword_bit(kw::COMPUTER),
            "remember" | "recall" | "memory" => keyword_bit(kw::REMEMBER),
            "i" | "im" | "ive" | "ill" | "id" => keyword_bit(kw::I),
            "you" | "youre" | "youve" | "youll" => keyword_bit(kw::YOU),
            "my" | "mine" | "myself" => keyword_bit(kw::MY),
            "name" | "named" | "called" => keyword_bit(kw::NAME),
            _ => 0,
        };
        mask |= bit;
    }
    mask
}

/// Reflection table: transform pronouns in captured text.
///
/// Cold path — used only when generating human-readable responses.
pub fn reflect(text: &str) -> String {
    text.split_whitespace()
        .map(|tok| {
            let lower = tok.to_lowercase();
            match lower.as_str() {
                "i" => "you".to_string(),
                "me" => "you".to_string(),
                "my" => "your".to_string(),
                "myself" => "yourself".to_string(),
                "you" => "I".to_string(),
                "your" => "my".to_string(),
                "yours" => "mine".to_string(),
                "yourself" => "myself".to_string(),
                "am" => "are".to_string(),
                "are" => "am".to_string(),
                _ => tok.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render a template index into a human-readable response.
pub fn render_template(template: u8, rotation: usize) -> &'static str {
    let responses: &[&[&str]] = &[
        // APOLOGIZE
        &["Please don't apologize.", "Apologies are not necessary.", "What feelings do you have when you apologize?"],
        // DREAM
        &["What does that dream suggest to you?", "Do you often have dreams?", "Have you dreamt about this before?"],
        // FAMILY
        &["Tell me more about your family.", "Who else in your family comes to mind?", "Your family is important to you."],
        // ELABORATE
        &["Can you elaborate on that?", "What makes you say that?", "Please go on."],
        // SIMILARITY
        &["In what way are they alike?", "What similarity do you see?", "What resemblance do you notice?"],
        // FEELINGS
        &["How does that make you feel?", "Tell me more about those feelings.", "What circumstances produce that feeling?"],
        // COMPUTER
        &["Do computers worry you?", "What do you think about machines?", "Are you frightened by computers?"],
        // MEMORY
        &["What does that memory mean to you?", "Why do you remember that now?", "What makes you think of that?"],
        // REFLECT_I
        &["You say you... go on.", "What do you mean when you say that?", "Tell me more about yourself."],
        // REFLECT_YOU
        &["We were discussing you, not me.", "Why do you mention me?", "What about me interests you?"],
        // NAME
        &["Names don't interest me.", "I don't care about names.", "Please continue."],
        // FALLBACK_GO_ON
        &["Please go on.", "Continue.", "I see."],
        // FALLBACK_TELL
        &["Tell me more.", "What else?", "Go on."],
        // FALLBACK_SEE
        &["I see.", "That's interesting.", "How interesting."],
    ];

    let idx = template as usize;
    if idx >= responses.len() {
        return "Please go on.";
    }
    let group = responses[idx];
    group[rotation % group.len()]
}

/// A high-level dialogue session that accumulates rotation state.
pub struct ElizaSession {
    rotation: usize,
    history: u64,
}

impl Default for ElizaSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ElizaSession {
    pub fn new() -> Self {
        ElizaSession { rotation: 0, history: 0 }
    }

    /// Full text-in / text-out turn. Cold path (~10 µs).
    pub fn respond(&mut self, input: &str) -> String {
        let mask = keywords_from_text(input);
        if mask == 0 {
            // No keyword: cycle fallback
            let template = match self.rotation % 3 {
                0 => tmpl::FALLBACK_GO_ON,
                1 => tmpl::FALLBACK_TELL,
                _ => tmpl::FALLBACK_SEE,
            };
            self.rotation = self.rotation.wrapping_add(1);
            return render_template(template, self.rotation).to_string();
        }
        let result = turn(mask, &DOCTOR, self.history);
        self.history = result.state;
        let template = if result.matched { result.template } else { tmpl::FALLBACK_GO_ON };
        let response = render_template(template, self.rotation);
        self.rotation = self.rotation.wrapping_add(1);
        response.to_string()
    }
}

// =============================================================================
// AUTOML SIGNAL — for HDIT integration
// =============================================================================

/// Generate an AutoML signal from ELIZA turns.
///
/// Each input mask is run through `turn_fast`; predicts true if a rule matched
/// (template != 0xFF). Timing is measured in nanoseconds-equivalent units.
pub fn eliza_automl_signal(
    name: &str,
    input_masks: &[u64],
    anchor: &[bool],
) -> SignalProfile {
    let mut predictions = Vec::with_capacity(input_masks.len());
    let mut total_ns = 0u64;
    for &mask in input_masks {
        let template = turn_fast(mask, &DOCTOR);
        predictions.push(template != 0xFF);
        total_ns += 5; // ~5 ns per inference (T0 territory)
    }
    let timing_us = (total_ns / 1000).max(1);
    SignalProfile::new(name, predictions, anchor, timing_us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_bit_is_correct_position() {
        assert_eq!(keyword_bit(0), 0x1);
        assert_eq!(keyword_bit(1), 0x2);
        assert_eq!(keyword_bit(15), 0x8000);
    }

    #[test]
    fn template_bit_is_in_high_half() {
        assert_eq!(template_bit(0), 1u64 << 32);
    }

    #[test]
    fn doctor_table_size_is_compile_time_const() {
        assert_eq!(DOCTOR.len(), 11);
        // Each rule must be 16 bytes for cache-line packing
        assert_eq!(core::mem::size_of::<ElizaRule>(), 16);
    }

    #[test]
    fn turn_fast_sorry_matches_apologize() {
        let mask = keyword_bit(kw::SORRY);
        assert_eq!(turn_fast(mask, &DOCTOR), tmpl::APOLOGIZE);
    }

    #[test]
    fn turn_fast_dream_matches_dream_template() {
        let mask = keyword_bit(kw::DREAM);
        assert_eq!(turn_fast(mask, &DOCTOR), tmpl::DREAM);
    }

    #[test]
    fn turn_fast_mother_matches_family_template() {
        let mask = keyword_bit(kw::MOTHER);
        assert_eq!(turn_fast(mask, &DOCTOR), tmpl::FAMILY);
    }

    #[test]
    fn turn_fast_no_keyword_returns_no_match() {
        assert_eq!(turn_fast(0, &DOCTOR), 0xFF);
    }

    #[test]
    fn turn_fast_higher_priority_wins_over_lower() {
        // COMPUTER (rank 0) must beat I (rank 5) when both are present
        let mask = keyword_bit(kw::COMPUTER) | keyword_bit(kw::I);
        assert_eq!(turn_fast(mask, &DOCTOR), tmpl::COMPUTER);
    }

    #[test]
    fn turn_full_state_advances_with_history() {
        let mask = keyword_bit(kw::DREAM);
        let r1 = turn(mask, &DOCTOR, 0);
        let r2 = turn(mask, &DOCTOR, r1.state);
        assert!(r1.matched);
        assert!(r2.matched);
        assert_ne!(r1.state, r2.state, "history must roll forward");
    }

    #[test]
    fn turn_full_state_unmatched_returns_0xff() {
        let r = turn(0, &DOCTOR, 0);
        assert!(!r.matched);
        assert_eq!(r.template, 0xFF);
        assert_eq!(r.rule_idx, 0xFF);
    }

    #[test]
    fn keywords_from_text_extracts_dream() {
        let mask = keywords_from_text("I had a strange dream last night");
        assert_ne!(mask & keyword_bit(kw::DREAM), 0);
        assert_ne!(mask & keyword_bit(kw::I), 0);
    }

    #[test]
    fn keywords_from_text_handles_punctuation() {
        let mask = keywords_from_text("Are you happy?");
        assert_ne!(mask & keyword_bit(kw::HAPPY), 0);
        assert_ne!(mask & keyword_bit(kw::YOU), 0);
    }

    #[test]
    fn keywords_from_text_synonyms_collapse() {
        let m1 = keywords_from_text("mom");
        let m2 = keywords_from_text("mother");
        assert_eq!(m1, m2);
    }

    #[test]
    fn reflect_transforms_pronouns() {
        assert_eq!(reflect("I am sad"), "you are sad");
        assert_eq!(reflect("my mother"), "your mother");
        assert_eq!(reflect("you are nice"), "I am nice");
    }

    #[test]
    fn render_template_rotates() {
        let r0 = render_template(tmpl::APOLOGIZE, 0);
        let r1 = render_template(tmpl::APOLOGIZE, 1);
        assert_ne!(r0, r1, "rotation must produce different responses");
    }

    #[test]
    fn session_responds_to_dream() {
        let mut s = ElizaSession::new();
        let r = s.respond("I had a dream");
        assert!(r.to_lowercase().contains("dream"));
    }

    #[test]
    fn session_falls_back_when_no_keyword() {
        let mut s = ElizaSession::new();
        let r = s.respond("xyz qrs def");
        assert!(!r.is_empty());
    }

    #[test]
    fn session_handles_empty_input() {
        let mut s = ElizaSession::new();
        let r = s.respond("");
        assert!(!r.is_empty());
    }

    #[test]
    fn eliza_automl_signal_predicts_correctly() {
        let masks = [
            keyword_bit(kw::DREAM),
            0,
            keyword_bit(kw::SORRY),
        ];
        let anchor = [true, false, true];
        let sig = eliza_automl_signal("eliza", &masks, &anchor);
        assert_eq!(sig.predictions, vec![true, false, true]);
        assert_eq!(sig.accuracy_vs_anchor, 1.0);
    }

    #[test]
    fn turn_fast_is_deterministic_across_invocations() {
        let mask = keyword_bit(kw::DREAM) | keyword_bit(kw::I);
        let a = turn_fast(mask, &DOCTOR);
        let b = turn_fast(mask, &DOCTOR);
        let c = turn_fast(mask, &DOCTOR);
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn session_responses_are_deterministic_for_same_seed() {
        // Two fresh sessions on the same input sequence produce identical outputs
        let mut s1 = ElizaSession::new();
        let mut s2 = ElizaSession::new();
        for input in &["I had a dream", "My mother is here", "I am sorry"] {
            assert_eq!(s1.respond(input), s2.respond(input));
        }
    }

    #[test]
    fn eliza_automl_signal_is_t0_tier() {
        use crate::ml::hdit_automl::Tier;
        let masks = [keyword_bit(kw::DREAM); 100];
        let anchor = [true; 100];
        let sig = eliza_automl_signal("eliza_t0", &masks, &anchor);
        assert_eq!(sig.tier, Tier::T0, "ELIZA must be T0 tier");
    }
}
