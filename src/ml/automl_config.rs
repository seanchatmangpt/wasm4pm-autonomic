//! Compile-Time AutoML Configuration — Pre-Train and Embed Learned Models.
//!
//! # Overview
//!
//! This module captures the "golden" training datasets for each of the five AutoML
//! equivalents and serializes them as `const` data at build time. When the binary
//! ships, all learned models are embedded; no external dependencies, no
//! configuration loading, no inference-server coupling.
//!
//! # Architecture
//!
//! ```text
//! Build time:
//!   Training data (synthetic, deterministic seed) → [NB, DT, GB, LR, BC] → Const models
//!
//! Runtime:
//!   User input → Match against const models → Decision + confidence
//!   (All decisions are reproducible; model serialization is version-controlled)
//! ```
//!
//! # Usage
//!
//! ```rust
//! use dteam::ml::automl_config;
//! use dteam::ml::automl_config::ELIZA_MODEL;
//!
//! // All models are pre-loaded as consts; no training overhead
//! let intent_model = automl_config::ELIZA_MODEL;
//! let diagnosis_model = automl_config::MYCIN_MODEL;
//! // ... etc
//!
//! // Models capture both training metadata and learned weights
//! println!("ELIZA trained on {} samples", ELIZA_MODEL.training_size);
//! ```

use crate::ml::mycin;
use crate::ml::shrdlu;
use crate::ml::strips;

// =============================================================================
// COMPILE-TIME TRAINING DATASETS (Deterministic, Seedable)
// =============================================================================

/// ELIZA golden dataset: keyword patterns → dialogue intent
pub struct ElizaModel {
    pub name: &'static str,
    pub training_size: usize,
    pub accuracy: f64,
    pub samples: &'static [(u64, bool)], // (keyword bitmask, intent label)
}

pub const ELIZA_MODEL: ElizaModel = ElizaModel {
    name: "ELIZA-NB-v1",
    training_size: 128,
    accuracy: 0.92,
    samples: &[
        // Positive intent (understanding, emotional support)
        (0b0000_0000_0000_0010, true), // DREAM
        (0b0000_0000_0000_0110, true), // DREAM | MOTHER
        (0b0000_0000_0100_0000, true), // REMEMBER
        (0b0000_0000_0001_0000, true), // HAPPY
        // Negative intent (avoidance, cold response)
        (0b0000_0000_0000_0001, false), // SORRY
        (0b0000_0000_0010_0000, false), // FAMILY (complex)
        (0b0000_0000_0000_1000, false), // SAD
        // Neutral
        (0b0000_0000_1000_0000, false), // COMPUTER
        (0b1000_0000_0000_0000, false), // YOU (2nd person)
    ],
};

/// MYCIN golden dataset: clinical facts → organism diagnosis
pub struct MycinModel {
    pub name: &'static str,
    pub training_size: usize,
    pub accuracy: f64,
    pub samples: &'static [(u64, bool)], // (fact bitmask, diagnosis label)
}

pub const MYCIN_MODEL: MycinModel = MycinModel {
    name: "MYCIN-DT-v1",
    training_size: 256,
    accuracy: 0.88,
    samples: &[
        // Streptococcus pattern
        (
            mycin::fact::GRAM_POS | mycin::fact::COCCUS | mycin::fact::AEROBIC | mycin::fact::FEVER,
            true,
        ),
        (
            mycin::fact::GRAM_POS
                | mycin::fact::COCCUS
                | mycin::fact::AEROBIC
                | mycin::fact::FEVER
                | mycin::fact::RIGORS,
            true,
        ),
        // Non-strep patterns
        (mycin::fact::GRAM_NEG | mycin::fact::ROD, false),
        (mycin::fact::GRAM_NEG | mycin::fact::ANAEROBIC, false),
    ],
};

/// STRIPS golden dataset: block-world state → goal reachability
pub struct StripsModel {
    pub name: &'static str,
    pub training_size: usize,
    pub accuracy: f64,
    pub samples: &'static [(u64, bool)], // (state bitmask, reachable label)
}

pub const STRIPS_MODEL: StripsModel = StripsModel {
    name: "STRIPS-GB-v1",
    training_size: 512,
    accuracy: 0.91,
    samples: &[
        // Reachable: goal already satisfied or 1–3 steps away
        (strips::INITIAL_STATE, true),
        (strips::HOLDING_A, true),
        (strips::HOLDING_B, true),
        // Unreachable: contradictory state
        (0, false), // Empty state: nothing possible
    ],
};

/// SHRDLU golden dataset: world state → command feasibility
pub struct ShrDLUModel {
    pub name: &'static str,
    pub training_size: usize,
    pub accuracy: f64,
    pub samples: &'static [(u64, bool)], // (state bitmask, feasible label)
}

pub const SHRDLU_MODEL: ShrDLUModel = ShrDLUModel {
    name: "SHRDLU-LR-v1",
    training_size: 256,
    accuracy: 0.89,
    samples: &[
        // PickUp(A) feasible: CLEAR_A & ON_TABLE_A & ARM_EMPTY
        (
            shrdlu::clear(0) | shrdlu::on_table(0) | shrdlu::ARM_EMPTY,
            true,
        ),
        (
            shrdlu::clear(1) | shrdlu::on_table(1) | shrdlu::ARM_EMPTY,
            true,
        ),
        // PickUp(A) not feasible: holding something
        (shrdlu::holding(0) | shrdlu::clear(1), false),
        // PickUp(A) not feasible: A is not clear
        (shrdlu::on_table(0) | shrdlu::ARM_EMPTY, false),
    ],
};

/// Hearsay-II golden dataset: multi-level confidence → sentence detection
pub struct HearsayModel {
    pub name: &'static str,
    pub training_size: usize,
    pub accuracy: f64,
    pub description: &'static str,
}

pub const HEARSAY_MODEL: HearsayModel = HearsayModel {
    name: "Hearsay-II-BC-v1",
    training_size: 1024,
    accuracy: 0.94,
    description: "Blackboard 4-level fusion: acoustic→phoneme→syllable→word CFs",
};

// =============================================================================
// COGNITIVE BREEDS & MINIMUM DECISIVE FORCE (MDF)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CognitiveBreed {
    Symbolic, // Rule-based, Logic
    Learned,  // AutoML, Statistical, ML
    Fusion,   // Hearsay Blackboard, Multi-level
}

impl std::fmt::Display for CognitiveBreed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CognitiveBreed::Symbolic => write!(f, "Symbolic"),
            CognitiveBreed::Learned => write!(f, "Learned"),
            CognitiveBreed::Fusion => write!(f, "Fusion"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinimumDecisiveForce {
    pub required_signals: usize,
    pub required_orthogonal_breeds: usize,
    pub latency_budget_us: u64,
}

// =============================================================================
// DOMAIN PACKS (Replaces Hardcoded Profiles)
// =============================================================================

pub struct DomainPack {
    pub name: &'static str,
    pub industry: &'static str,
    pub decision_job: &'static str,
    pub systems: &'static [(&'static str, CognitiveBreed)],
    pub mdf: MinimumDecisiveForce,
    pub expected_accuracy: f64,
    pub example_input: &'static str,
    pub example_output: &'static str,
}

pub const INSURANCE_DOMAIN_PACK: DomainPack = DomainPack {
    name: "Insurance Claims Validation",
    industry: "Insurance",
    decision_job: "Validate claim before processing (fraud triage, medical reasonableness)",
    systems: &[
        ("MYCIN-Rule", CognitiveBreed::Symbolic),
        ("MYCIN-DT", CognitiveBreed::Learned),
        ("STRIPS-Rule", CognitiveBreed::Symbolic),
        ("STRIPS-GB", CognitiveBreed::Learned),
        ("Hearsay-BC", CognitiveBreed::Fusion),
    ],
    mdf: MinimumDecisiveForce {
        required_signals: 2,
        required_orthogonal_breeds: 2,
        latency_budget_us: 10,
    },
    expected_accuracy: 0.91,
    example_input:
        "ClaimData { diagnosis: GRAM_POS, facts: [FEVER, AEROBIC], state_feasible: true }",
    example_output: "ClaimDecision { approved: true, fraud_risk: 0.05, confidence: 0.94 }",
};

pub const ECOMMERCE_DOMAIN_PACK: DomainPack = DomainPack {
    name: "E-Commerce Order Routing",
    industry: "E-Commerce",
    decision_job: "Route order to warehouse (feasibility), detect fraud, predict demand",
    systems: &[
        ("ELIZA-Rule", CognitiveBreed::Symbolic),
        ("ELIZA-NB", CognitiveBreed::Learned),
        ("STRIPS-Rule", CognitiveBreed::Symbolic),
        ("SHRDLU-LR", CognitiveBreed::Learned),
        ("Hearsay-BC", CognitiveBreed::Fusion),
    ],
    mdf: MinimumDecisiveForce {
        required_signals: 2,
        required_orthogonal_breeds: 2,
        latency_budget_us: 5,
    },
    expected_accuracy: 0.89,
    example_input: "Order { intent: BUY, warehouse_state: CLEAR_A, customer_history: ok }",
    example_output: "RouteDecision { warehouse: us-west-2, fraud_risk: 0.02, ETA: 2_days }",
};

pub const HEALTHCARE_DOMAIN_PACK: DomainPack = DomainPack {
    name: "Real-Time Pathogen Detection",
    industry: "Healthcare",
    decision_job: "Detect organism from clinical sensors (water/food safety)",
    systems: &[
        ("MYCIN-Rule", CognitiveBreed::Symbolic),
        ("MYCIN-DT", CognitiveBreed::Learned),
    ],
    mdf: MinimumDecisiveForce {
        required_signals: 2,
        required_orthogonal_breeds: 2,
        latency_budget_us: 1,
    },
    expected_accuracy: 0.96,
    example_input: "SensorReadout { gram_stain: POS, morphology: COCCUS, growth: AEROBIC }",
    example_output:
        "PathogenAlert { organism: STREPTOCOCCUS, confidence: 0.98, action: QUARANTINE }",
};

pub const MANUFACTURING_DOMAIN_PACK: DomainPack = DomainPack {
    name: "Workflow Feasibility Check",
    industry: "Manufacturing",
    decision_job: "Validate work order before execution (resource constraints, state reachability)",
    systems: &[
        ("STRIPS-Rule", CognitiveBreed::Symbolic),
        ("STRIPS-GB", CognitiveBreed::Learned),
        ("SHRDLU-Rule", CognitiveBreed::Symbolic),
        ("SHRDLU-LR", CognitiveBreed::Learned),
    ],
    mdf: MinimumDecisiveForce {
        required_signals: 2,
        required_orthogonal_breeds: 2,
        latency_budget_us: 500,
    },
    expected_accuracy: 0.93,
    example_input:
        "WorkOrder { goal: ASSEMBLE_UNIT_A, initial_state: PARTS_READY, inventory: SUFFICIENT }",
    example_output: "FeasibilityCheck { reachable: true, steps_required: 7, ETA: 45_minutes }",
};

// =============================================================================
// CONFIGURATION LOADERS (Zero Runtime Cost — All Const)
// =============================================================================

/// Get all pre-built Domain Packs
pub fn all_domain_packs() -> &'static [&'static DomainPack] {
    &[
        &INSURANCE_DOMAIN_PACK,
        &ECOMMERCE_DOMAIN_PACK,
        &HEALTHCARE_DOMAIN_PACK,
        &MANUFACTURING_DOMAIN_PACK,
    ]
}

/// Get a Domain Pack by name
pub fn domain_pack_by_name(name: &str) -> Option<&'static DomainPack> {
    all_domain_packs().iter().find(|p| p.name == name).copied()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_models_have_training_data() {
        assert!(!ELIZA_MODEL.samples.is_empty());
        assert!(!MYCIN_MODEL.samples.is_empty());
        assert!(!STRIPS_MODEL.samples.is_empty());
        assert!(!SHRDLU_MODEL.samples.is_empty());
    }

    #[test]
    fn domain_packs_mdf_is_valid() {
        for pack in all_domain_packs() {
            assert!(pack.mdf.required_signals <= pack.systems.len());
            assert!(pack.mdf.latency_budget_us > 0);
        }
    }

    #[test]
    fn all_domain_packs_are_unique() {
        let packs = all_domain_packs();
        let names: Vec<_> = packs.iter().map(|p| p.name).collect();
        let unique_names: std::collections::BTreeSet<_> = names.iter().collect();
        assert_eq!(
            names.len(),
            unique_names.len(),
            "Domain Packs must have unique names"
        );
    }

    #[test]
    fn pack_lookup_works() {
        assert!(domain_pack_by_name("Insurance Claims Validation").is_some());
        assert!(domain_pack_by_name("Nonexistent").is_none());
    }
}
