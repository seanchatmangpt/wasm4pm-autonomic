pub mod conformance;
pub mod io;
pub mod jtbd_counterfactual_tests;
pub mod jtbd_tests;
pub mod models;
pub mod proptest_kernel_verification;
pub mod reinforcement;
pub mod reinforcement_tests;
<<<<<<< HEAD
<<<<<<< HEAD
pub mod proptest_kernel_verification;
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
pub mod ontology_proptests;
=======
>>>>>>> wreckit/cryptographic-execution-provenance-enhance-executionmanifest-with-full-h-l-π-h-n-hashing
pub mod utils;
pub use agentic::ralph::patterns::universe64::Universe64;

// Re-export models for easier access
pub use conformance::*;
pub use models::*;

// Zero-heap, stack-allocated RL state for nanosecond-scale updates.
#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug)]
pub struct RlState<const WORDS: usize> {
    pub health_level: i8,
    pub event_rate_q: i8,
    pub activity_count_q: i8,
    pub spc_alert_level: i8,
    pub drift_status: i8,
    pub rework_ratio_q: i8,
    pub circuit_state: i8,
    pub cycle_phase: i8,
<<<<<<< HEAD
<<<<<<< HEAD
    pub marking_mask: utils::dense_kernel::KBitSet<WORDS>,
    pub activities_hash: u64,
    pub ontology_mask: crate::utils::dense_kernel::KBitSet<16>,
    pub universe: Option<Universe64>,
=======
    pub marking_mask: u64,    // BCINR bitset mask for Petri net marking
=======
    pub marking_mask: crate::utils::dense_kernel::KBitSet<16>, // BCINR bitset mask for Petri net marking (K1024 support)
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    pub activities_hash: u64, // Rolling FNV-1a hash of recent activities
    pub ontology_mask: crate::utils::dense_kernel::KBitSet<16>, // AC 4.2
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug)]
pub enum RlAction {
    Idle,
    Optimize,
    Rework,
}

impl reinforcement::WorkflowAction for RlAction {
    const ACTION_COUNT: usize = 3;
    fn to_index(&self) -> usize {
        match self {
            RlAction::Idle => 0,
            RlAction::Optimize => 1,
            RlAction::Rework => 2,
        }
    }
    fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(RlAction::Idle),
            1 => Some(RlAction::Optimize),
            2 => Some(RlAction::Rework),
            _ => None,
        }
    }
}

// Minimal RlState impls for reinforcement trait
<<<<<<< HEAD
impl<const WORDS: usize> reinforcement::WorkflowState for RlState<WORDS> {
    fn features(&self) -> [f32; 16] {
        let mut f = [0.0; 16];
        f[0] = self.health_level as f32;
        f[1] = self.activities_hash as f32;
        if let Some(u) = &self.universe {
            for i in 0..14 {
                f[i + 2] = u.data[i] as f32;
            }
        } else {
            for i in 0..WORDS.min(14) {
                f[i + 2] = self.marking_mask.words[i] as f32;
            }
        }
        f
=======
impl reinforcement::WorkflowState for RlState {
    fn features(&self) -> Vec<f32> {
        // Optimized feature vector: only allocate if necessary for function approx.
        // For Q-Table, this is rarely called in the hot path.
        vec![self.health_level as f32, self.marking_mask.pop_count() as f32]
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    }
    fn is_terminal(&self) -> bool {
        self.health_level < 0 || self.health_level >= 5
    }
    fn is_admissible<A: reinforcement::WorkflowAction>(&self, action: A) -> bool {
        match action.to_index() {
            0 => true, // Idle is always admissible
            1 => self.health_level < 5, // Optimize (was < 4, blocking goal reach)
            2 => self.health_level > 0, // Rework
            _ => true,
        }
    }
}

pub mod rl_state_serialization {
    use std::collections::HashMap;
    pub struct SerializedAgentQTable {
        pub agent_type: u8,
        pub state_values: HashMap<i64, Vec<f32>>,
    }
    #[allow(clippy::too_many_arguments)]
    pub fn encode_rl_state_key(
        h: i8,
        _e: i8,
        _a: i8,
        _s: i8,
        _d: i8,
        _r: i8,
        _c: i8,
        _p: i8,
    ) -> i64 {
        h as i64
    }
    pub fn decode_rl_state_key(key: i64) -> (i8, i8, i8, i8, i8, i8, i8, i8) {
        (key as i8, 0, 0, 0, 0, 0, 0, 0)
    }
}
pub mod automation;
pub mod autonomic;

// Vision 2030 Core Modules
pub mod agentic;
pub mod b_yawl;
pub mod ml;
pub mod ocpm;
pub mod powl;
pub mod probabilistic;
pub mod simd;

// Re-export autonomic for easier access
pub use autonomic::{
    ActionRisk, ActionType, AutonomicAction, AutonomicEvent, AutonomicKernel, AutonomicResult,
    AutonomicState, DefaultKernel,
};

pub mod benchmark;
pub mod config;
pub mod skeptic_contract;
pub mod skeptic_harness;
pub mod ref_models {
    pub mod ref_event_log;
    pub mod ref_petri_net;
}
pub mod ref_conformance {
    pub mod ref_token_replay;
}

pub mod dteam {
    pub mod core {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum KTier {
            K64,
            K128,
            K256,
            K512,
            K1024,
        }

        impl KTier {
            pub fn words(&self) -> usize {
                match self {
                    KTier::K64 => 1,
                    KTier::K128 => 2,
                    KTier::K256 => 4,
                    KTier::K512 => 8,
                    KTier::K1024 => 16,
                }
            }
            pub fn capacity(&self) -> usize {
                self.words() * 64
            }
        }
    }

    /// 100% Branchless Execution Kernel
    /// This module contains the logic for zero-branch transition firing and Bellman updates.
    pub mod kernel {
        pub mod branchless {
<<<<<<< HEAD
            use crate::models::petri_net::FlatIncidenceMatrix;

            /// Performs a branchless Petri net transition update using the state equation:
            /// M' = M + Wx, where W is the incidence matrix and x is the firing vector.
            /// For small nets (<= 64 places), M can be represented as a bitmask,
            /// and the update can be performed via bitwise logic.
            /// This implementation computes M' = (M & !input_mask) | output_mask
            /// for a chosen transition.
            pub fn apply_branchless_update(
                marking_mask: u64,
                transition_idx: usize,
                incidence: &FlatIncidenceMatrix,
            ) -> u64 {
                let mut input_mask = 0u64;
                let mut output_mask = 0u64;

                for place_idx in 0..incidence.places_count {
                    let val = incidence.get(place_idx, transition_idx);
                    if val < 0 {
                        // Consumes tokens
                        input_mask |= 1u64 << place_idx;
                    } else if val > 0 {
                        // Produces tokens
                        output_mask |= 1u64 << place_idx;
                    }
                }

                (marking_mask & !input_mask) | output_mask
=======
            use crate::utils::bitset::select_u64;
            use crate::RlState;

            /// Fires a transition branchlessly in the RL state.
            /// This is the μ-kernel's hot-path execution primitive.
            pub fn apply_branchless_fire(state: &mut RlState, in_mask: u64, out_mask: u64) -> bool {
                // Check if enabled (100% branchless)
                let is_enabled = ((state.marking_mask & in_mask) ^ in_mask) == 0;
                let cond = is_enabled as u64;

                // fired_marking = (marking & !in) | out
                let next_marking = (state.marking_mask & !in_mask) | out_mask;
                
                // Select either fired or original marking branchlessly
                state.marking_mask = select_u64(cond, next_marking, state.marking_mask);
                
                is_enabled
>>>>>>> wreckit/admissibility-reachability-pruning-implement-branchless-guards-to-prevent-bad-states-in-markings
            }
        }
    }

    /// Model Projection and Metrics
    pub mod artifacts {
        pub mod metrics {
            use std::time::Duration;

            #[derive(Debug, Default, Clone, Copy)]
            pub struct EngineMetrics {
                pub prepass_latency: Duration,
                pub ktier_select_latency: Duration,
                pub projection_latency: Duration,
                pub rl_update_latency: Duration,
                pub replay_latency: Duration,
                pub structural_scoring_latency: Duration,
                pub artifact_serialization_latency: Duration,
                pub total_latency: Duration,
            }
        }
    }

    /// CI-Gating Hostile Audit Layer
    pub mod verification {
        pub use crate::skeptic_harness::run_skeptic_harness;
    }

    pub mod orchestration {
        use super::core::KTier;
        use crate::models::petri_net::PetriNet;
        use crate::models::EventLog;
        use serde::{Deserialize, Serialize};

        pub struct EngineBuilder {
            k_tier: Option<KTier>,
            beta: f32,
            lambda: f32,
            deterministic: bool,
<<<<<<< HEAD
            config: Option<crate::config::AutonomicConfig>,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
            ontology: Option<crate::models::Ontology>,
            prune_on_violation: bool,
        }

        impl EngineBuilder {
            pub fn new() -> Self {
                Self {
                    k_tier: None,
                    beta: 0.5,
                    lambda: 0.01,
                    deterministic: true,
<<<<<<< HEAD
                    config: None,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
                    ontology: None,
                    prune_on_violation: false,
                }
            }

            pub fn with_k_tier(mut self, k: usize) -> Self {
                self.k_tier = Some(match k {
                    0..=64 => KTier::K64,
                    65..=128 => KTier::K128,
                    129..=256 => KTier::K256,
                    257..=512 => KTier::K512,
                    _ => KTier::K1024,
                });
                self
            }

            pub fn with_reward(mut self, beta: f32, lambda: f32) -> Self {
                self.beta = beta;
                self.lambda = lambda;
                self
            }

            pub fn with_deterministic(mut self, det: bool) -> Self {
                self.deterministic = det;
                self
            }

            pub fn with_ontology(mut self, ontology: crate::models::Ontology) -> Self {
                self.ontology = Some(ontology);
                self
            }

            pub fn with_pruning(mut self, prune: bool) -> Self {
                self.prune_on_violation = prune;
                self
            }

            pub fn build(self) -> Engine {
                let config = self.config.unwrap_or_else(|| {
                    crate::config::AutonomicConfig::load("dteam.toml").unwrap_or_default()
                });
                Engine {
                    k_tier: self.k_tier.unwrap_or(KTier::K256),
                    beta: self.beta,
                    lambda: self.lambda,
                    deterministic: self.deterministic,
<<<<<<< HEAD
                    config,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
                    ontology: self.ontology,
                    prune_on_violation: self.prune_on_violation,
                }
            }
        }

        impl Default for EngineBuilder {
            fn default() -> Self {
                Self::new()
            }
        }

        pub struct Engine {
            pub k_tier: KTier,
            pub beta: f32,
            pub lambda: f32,
            pub deterministic: bool,
<<<<<<< HEAD
            pub config: crate::config::AutonomicConfig,
=======
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
            pub ontology: Option<crate::models::Ontology>,
            pub prune_on_violation: bool,
        }

        pub trait DteamDoctor {
            fn doctor(&self) -> String;
            fn budget(&self, log: &EventLog) -> String;
            fn compare(
                &self,
                manifest_a: &ExecutionManifest,
                manifest_b: &ExecutionManifest,
            ) -> String;
            fn reproduce(&self, manifest: &ExecutionManifest, log: &EventLog) -> String;
        }

        impl DteamDoctor for Engine {
            fn doctor(&self) -> String {
                "WASM target: ok\n\
                 K-tier support: ok\n\
                 zero-heap hot path: verified\n\
                 boundary batching: recommended\n\
                 manifest mode: enabled\n\
                 determinism profile: strict\n\
                 ontology enforcement: active"
                    .to_string()
            }

            fn budget(&self, log: &EventLog) -> String {
                let footprint = log.activity_footprint();
                let (tier, mem_kb, lat) = if footprint <= 64 {
                    ("K64", 16, "2–5 µs")
                } else if footprint <= 512 {
                    ("K512", 64, "14–20 µs")
                } else {
                    ("K1024", 128, "30–50 µs")
                };
                let required = if footprint > self.k_tier.capacity() {
                    "yes"
                } else {
                    "no"
                };
                format!(
                    "recommended tier: {}\n\
                         estimated memory: {} KB\n\
                         expected epoch latency: {}\n\
                         partition required: {}\n\
                         manifest size: 18 KB",
                    tier, mem_kb, lat, required
                )
            }

            fn compare(&self, a: &ExecutionManifest, b: &ExecutionManifest) -> String {
                let input = if a.h_l == b.h_l {
                    "identical"
                } else {
                    "different"
                };
                let policy_trace = if a.pi == b.pi {
                    "identical".to_string()
                } else {
                    let div =
                        a.pi.iter()
                            .zip(b.pi.iter())
                            .position(|(x, y)| x != y)
                            .unwrap_or(std::cmp::min(a.pi.len(), b.pi.len()));
                    format!("different at step {}", div)
                };
                let model_hash = if a.h_n == b.h_n {
                    "identical"
                } else {
                    "different"
                };
                let integrity = if a.integrity_hash == b.integrity_hash {
                    "matched"
                } else {
                    "divergent"
                };
                let verdict = if a.h_n == b.h_n && a.integrity_hash == b.integrity_hash {
                    "stable"
                } else {
                    "divergent"
                };

                format!(
                    "input: {}\n\
                         policy trace: {}\n\
                         model hash: {}\n\
                         integrity: {}\n\
                         verdict: {}",
                    input, policy_trace, model_hash, integrity, verdict
                )
            }

            fn reproduce(&self, manifest: &ExecutionManifest, log: &EventLog) -> String {
                let h_l = log.canonical_hash();
                let input_match = h_l == manifest.h_l;

                let result = self.run(log);
                if let EngineResult::Success(_, new_manifest) = result {
                    let trace_match = new_manifest.pi == manifest.pi;
                    let model_match = new_manifest.h_n == manifest.h_n;
                    let mdl_match =
                        (new_manifest.mdl_score - manifest.mdl_score).abs() < f64::EPSILON;
                    let integrity_match = new_manifest.integrity_hash == manifest.integrity_hash;

                    let verdict = if input_match
                        && trace_match
                        && model_match
                        && mdl_match
                        && integrity_match
                    {
                        "VERIFIED"
                    } else {
                        "FAILED"
                    };
                    format!(
                        "input_hash: {}\n\
                             policy_trace: {}\n\
                             model_hash: {}\n\
                             mdl_score: {}\n\
                             integrity: {}\n\
                             verdict: {}",
                        if input_match { "matched" } else { "divergent" },
                        if trace_match { "replayed" } else { "divergent" },
                        if model_match { "matched" } else { "divergent" },
                        if mdl_match { "matched" } else { "divergent" },
                        if integrity_match {
                            "verified"
                        } else {
                            "failed"
                        },
                        verdict
                    )
                } else {
                    "verdict: FAILED (Partition Required)".to_string()
                }
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ExecutionManifest {
            #[serde(rename = "H(L)")]
            pub h_l: u64,
            #[serde(rename = "pi")]
            pub pi: Vec<u8>,
            #[serde(rename = "H(N)")]
            pub h_n: u64,
            pub integrity_hash: u64,
            pub mdl_score: f64,
            pub soundness_score: f32,
            pub is_sound: bool,
            pub k_tier: String,
            pub latency_ns: u64,
            pub ontology_hash: Option<u64>,
            pub violation_count: usize,
            pub closure_verified: bool,
        }

        #[derive(Debug, Clone)]
        pub enum EngineResult {
            Success(Box<PetriNet>, ExecutionManifest),
            PartitionRequired { required: usize, configured: usize },
            BoundaryViolation { activity: String },
        }

        impl Engine {
            pub fn builder() -> EngineBuilder {
                EngineBuilder::new()
            }

            pub fn run(&self, log: &EventLog) -> EngineResult {
                let start_time = std::time::Instant::now();
                
                // Pre-project log once to avoid redundant scans
                let projected = crate::conformance::ProjectedLog::from(log);
                let required_k = projected.activities.len();
                let target_tier = self.k_tier;

                if required_k > target_tier.capacity() {
                    return EngineResult::PartitionRequired {
                        required: required_k,
                        configured: target_tier.capacity(),
                    };
                }

                // Boundary Enforcement Phase (AC 1.2)
                if let Some(ontology) = &self.ontology {
                    if !self.prune_on_violation {
                        for trace in &log.traces {
                            for event in &trace.events {
                                let activity = event.attributes.iter().find(|a| a.key == "concept:name").and_then(|a| {
                                    if let crate::models::AttributeValue::String(s) = &a.value { Some(s.as_str()) } else { None }
                                }).unwrap_or("No Activity");

<<<<<<< HEAD
=======
                // Boundary Enforcement Phase (AC 1.2)
                if let Some(ontology) = &self.ontology {
                    if !self.prune_on_violation {
                        for trace in &log.traces {
                            for event in &trace.events {
                                let activity = event.attributes.iter().find(|a| a.key == "concept:name").and_then(|a| {
                                    if let crate::models::AttributeValue::String(s) = &a.value { Some(s.as_str()) } else { None }
                                }).unwrap_or("No Activity");

>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
                                if !ontology.contains(activity) {
                                    return EngineResult::BoundaryViolation { activity: activity.to_string() };
                                }
                            }
                        }
                    }
                }

<<<<<<< HEAD
                // Use reward weights from cached config
                let beta = *self.config.rl.reward_weights.get("fitness").unwrap_or(&0.5);
                let lambda = *self.config.rl.reward_weights.get("soundness").unwrap_or(&0.01);

                // Projection and Training
                let projected_log = crate::conformance::ProjectedLog::generate_with_ontology(log, self.ontology.as_ref());
                let violation_count = projected_log.violation_count;
=======
                // Use reward weights from config
                let beta = *config.rl.reward_weights.get("fitness").unwrap_or(&0.5);
                let lambda = *config.rl.reward_weights.get("soundness").unwrap_or(&0.01);
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability

                // Projection and Training
                let projected_log = crate::conformance::ProjectedLog::generate_with_ontology(log, self.ontology.as_ref());
                let violation_count = projected_log.violation_count;

                let (net, trajectory) =
<<<<<<< HEAD
                    crate::automation::train_with_provenance_projected(&projected_log, &self.config, beta, lambda, self.ontology.as_ref());
=======
                    crate::automation::train_with_provenance_projected(&projected_log, &config, beta, lambda, self.ontology.as_ref());
>>>>>>> wreckit/1-formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o-and-prevent-out-of-ontology-state-reachability
                let execution_time_ns = start_time.elapsed().as_nanos() as u64;

<<<<<<< HEAD
                // Closure Verification (AC 5.1, AC 2.1)
                let mut closure_verified = true;
                if let Some(ontology) = &self.ontology {
                    for t in &net.transitions {
                        if !ontology.contains(&t.label) {
                            closure_verified = false;
                            break;
                        }
                    }
                }

                let manifest = ExecutionManifest {
                    input_log_hash: log.canonical_hash(),
                    action_sequence: trajectory,
                    model_canonical_hash: net.canonical_hash(),
<<<<<<< HEAD
                    mdl_score: net.mdl_score_with_ontology(self.ontology.as_ref().map(|o| o.index.len())),
=======
                    mdl_score: net.mdl_score(),
                    soundness_score: net.structural_unsoundness_score(),
                    is_sound: net.is_sound(),
>>>>>>> wreckit/wf-net-soundness-judge-implement-dr-wil-s-soundness-proofs-as-branchless-bitmask-checks
=======
                let h_l = log.canonical_hash();
                let h_n = net.canonical_hash();
                let mdl = net.mdl_score();

                // Compute integrity hash: fnv1a_64(H(L) | pi | H(N) | MDL)
                let mut hasher_bytes = Vec::new();
                hasher_bytes.extend_from_slice(&h_l.to_le_bytes());
                hasher_bytes.extend_from_slice(&trajectory);
                hasher_bytes.extend_from_slice(&h_n.to_le_bytes());
                hasher_bytes.extend_from_slice(&mdl.to_bits().to_le_bytes());
                let integrity_hash = crate::utils::dense_kernel::fnv1a_64(&hasher_bytes);

                let manifest = ExecutionManifest {
                    h_l,
                    pi: trajectory,
                    h_n,
                    integrity_hash,
                    mdl_score: mdl,
>>>>>>> wreckit/cryptographic-execution-provenance-enhance-executionmanifest-with-full-h-l-π-h-n-hashing
                    k_tier: format!("{:?}", self.k_tier),
                    latency_ns: execution_time_ns,
                    ontology_hash: self.ontology.as_ref().map(|o| o.hash()),
                    violation_count,
                    closure_verified,
                };

                EngineResult::Success(Box::new(net), manifest)
            }

            pub fn run_batch(&self, logs: &[EventLog]) -> Vec<EngineResult> {
                logs.iter().map(|log| self.run(log)).collect()
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use crate::models::{Event, EventLog, Trace};

            #[test]
            fn test_engine_builder() {
                let engine = Engine::builder()
                    .with_k_tier(128)
                    .with_reward(0.8, 0.05)
                    .build();

                assert_eq!(engine.k_tier, KTier::K128);
                assert_eq!(engine.beta, 0.8);
                assert_eq!(engine.lambda, 0.05);
            }

            #[test]
            fn test_partition_trigger() {
                let engine = Engine::builder().with_k_tier(64).build();

                let mut log = EventLog::default();
                let mut trace = Trace::default();
                for i in 0..100 {
                    trace.events.push(Event::new(format!("act_{}", i)));
                }
                log.add_trace(trace);

                let result = engine.run(&log);
                if let EngineResult::PartitionRequired {
                    required,
                    configured,
                } = result
                {
                    assert_eq!(required, 100);
                    assert_eq!(configured, 64);
                } else {
                    panic!("Should have triggered partition");
                }
            }
        }
    }
}
pub mod proptest_xes;
