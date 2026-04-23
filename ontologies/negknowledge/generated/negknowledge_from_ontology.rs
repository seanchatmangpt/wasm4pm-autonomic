// GENERATED — do not edit by hand.
// Source: ontologies/negknowledge/ontology/negknowledge.ttl
// Template: ontologies/negknowledge/templates/negknowledge_table.njk
//
// Regenerate with `@unrdf/cli sync` from the ontologies/negknowledge
// directory (see README.md).

use unibit_negknowledge::{NegativeResult, Outcome};

/// Negative-knowledge entries derived from the ontology.
///
/// This is an ontology-driven companion to the hand-authored
/// `unibit_negknowledge::NEGATIVE_KNOWLEDGE`. Both tables share the
/// same `NegativeResult` shape; consumers can choose either.
pub const NEGATIVE_KNOWLEDGE_FROM_ONTOLOGY: &[NegativeResult] = &[
    NegativeResult {
        id: "CONDVAR_WORKER_POOL_SUB_MICROSECOND",
        attempt: "std::sync::Condvar-based worker pool for sub-µs fan-out",
        source: "docs/opus/58, docs/opus/59",
        outcome: Outcome::Pessimisation,
        reason: "Condvar wake-up is ~3 µs per notify; eight-core fan-out needs futex/ulock or spin-armed cores.",
    },
    NegativeResult {
        id: "CRITERION_ITER_WITH_SETUP_FOR_NS_OPS",
        attempt: "Criterion iter_with_setup to benchmark sub-100 ns operations",
        source: "docs/opus/58",
        outcome: Outcome::Pessimisation,
        reason: "Setup harness overhead dominates the measurement; use reused-state benches instead.",
    },
    NegativeResult {
        id: "HOT_HASHMAP_FOR_PROTOTYPE_LOOKUP",
        attempt: "HashMap<HyperVector, Prototype> on hot path",
        source: "doc 37 SPR, doc 49 glossary",
        outcome: Outcome::Rejected,
        reason: "Hash probing defeats locality; use PackedKeyTable / dense direct indexing.",
    },
    NegativeResult {
        id: "MANUAL_SUPEROP_FUSION",
        attempt: "manually fused admit+commit+fragment superop at T0",
        source: "docs/opus/58, docs/opus/59",
        outcome: Outcome::Pessimisation,
        reason: "Autovectoriser already fuses #[inline(always)] primitives; manual fusion adds state-passing overhead.",
    },
];
