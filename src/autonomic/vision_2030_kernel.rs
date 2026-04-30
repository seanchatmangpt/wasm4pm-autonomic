use crate::agentic::Simulator;
use crate::autonomic::{
    ActionRisk, ActionType, AutonomicAction, AutonomicEvent, AutonomicFeedback, AutonomicKernel,
    AutonomicResult, AutonomicState,
};
use crate::autonomic::bark::{BarkEvent, BarkKind};
use crate::autonomic::types::PackPosture;
use crate::config::AutonomicConfig;
use crate::io::prediction_log::PredictionLogBuffer;
use crate::ml::LinUcb;
use crate::ocpm::StreamingOcDfg;
use crate::powl::core::PowlModel;
use crate::probabilistic::CountMinSketch;
use crate::simd::SwarMarking;
use crate::utils::bitset::select_u64;
use crate::utils::dense_kernel::{fnv1a_64, KBitSet, PackedKeyTable};

/// Operational dimensions for the LinUCB bandit
const CONTEXT_DIM: usize = 10;
const CONTEXT_DIM_2: usize = 100;

// === AUTONOMIC CONSTANTS ===
const HEALTH_PENALTY_POWL_VIOLATION: f32 = 0.15;
const CONFORMANCE_PENALTY_SWAR_VIOLATION: f32 = 0.1;
const HEALTH_PENALTY_SWAR_VIOLATION: f32 = 0.05;
const OCPM_DIVERGENCE_THRESHOLD: u32 = 5;
const CONFORMANCE_REWARD_REPAIR: f32 = 0.2;
const HEALTH_REWARD_REPAIR: f32 = 0.1;
const CONFORMANCE_REWARD_RECOVER: f32 = 0.15;
const HEALTH_REWARD_RECOVER: f32 = 0.05;
const HEALTH_DECAY_NEGATIVE_REWARD: f32 = 0.02;
const HEALTH_IMPROVEMENT_POSITIVE_REWARD: f32 = 0.01;
const FNV_MIX_PRIME: u64 = 0x9E3779B185EBCA87;
const QUALIFIER_CREATES: u64 = 0xC1;
const QUALIFIER_UPDATES: u64 = 0xD1;
const QUALIFIER_READS: u64 = 0xE1;
const ITEM_TYPE_HASH: u64 = 0x1111;
const ORDER_TYPE_HASH: u64 = 0x2222;
const EDGE_MASK_4096: usize = 4095;

pub struct Vision2030Kernel<const WORDS: usize> {
    pub marking: SwarMarking<WORDS>,
    pub sketch: CountMinSketch,
    pub bandit: LinUcb<CONTEXT_DIM, CONTEXT_DIM_2>,
    pub oc_dfg: StreamingOcDfg<1024, 4096>,
    pub powl_model: PowlModel<WORDS>,
    pub config: AutonomicConfig,
    pub state: AutonomicState,

    // --- REAL PROCESS ENGINE DATA ---
    pub activity_table: PackedKeyTable<String, u8>,
    pub transition_inputs: Vec<[u64; WORDS]>,
    pub transition_outputs: Vec<[u64; WORDS]>,
    pub trace_buffer: [u8; 256],
    pub trace_cursor: usize,
    pub powl_executed_mask: KBitSet<WORDS>,
    pub powl_prev_idx: usize,

    // --- BANDIT CONTEXT REPLAY (Vision 2030) ---
    // last_context uses Cell for interior mutability inside &self propose()
    pub last_context: std::cell::Cell<[f32; CONTEXT_DIM]>,
    pub pre_action_conformance: f32,
    pub adaptation_count: u32,
    pub total_firings: u64,
    pub total_violations: u64,
    pub prediction_log: PredictionLogBuffer<64>,
}

impl<const WORDS: usize> Default for Vision2030Kernel<WORDS> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const WORDS: usize> Vision2030Kernel<WORDS> {
    pub fn new() -> Self {
        let config = AutonomicConfig::load("dteam.toml").unwrap_or_default();

        use crate::powl::core::{PowlNode, PowlOperator};

        let root = PowlNode::Operator {
            operator: PowlOperator::PARALLEL,
            children: vec![
                PowlNode::Operator {
                    operator: PowlOperator::SEQUENCE,
                    children: vec![
                        PowlNode::Transition {
                            label: Some("Start".to_string()),
                            id: 0,
                        },
                        PowlNode::Operator {
                            operator: PowlOperator::XOR,
                            children: vec![
                                PowlNode::Transition {
                                    label: Some("Normal".to_string()),
                                    id: 1,
                                },
                                PowlNode::Transition {
                                    label: Some("Bypass".to_string()),
                                    id: 2,
                                },
                            ],
                        },
                        PowlNode::Transition {
                            label: Some("End".to_string()),
                            id: 3,
                        },
                    ],
                },
                PowlNode::Transition {
                    label: Some("ConcurrentA".to_string()),
                    id: 4,
                },
                PowlNode::Transition {
                    label: Some("ConcurrentB".to_string()),
                    id: 5,
                },
            ],
        };

        let powl_model = PowlModel::new(root);
        let mut activity_table = PackedKeyTable::new();

        // Define activities
        let activities = [
            "Start",
            "Normal",
            "Bypass",
            "End",
            "ConcurrentA",
            "ConcurrentB",
        ];
        for (i, name) in activities.iter().enumerate() {
            activity_table.insert(fnv1a_64(name.as_bytes()), name.to_string(), i as u8);
        }

        // --- SWAR INCIDENCE CONFIGURATION ---
        let mut transition_inputs = vec![[0u64; WORDS]; PowlModel::<WORDS>::MAX_NODES];
        let mut transition_outputs = vec![[0u64; WORDS]; PowlModel::<WORDS>::MAX_NODES];

        // Model: p0 -> [Start] -> p1 -> [Normal/Bypass] -> p2 -> [End] -> p3
        transition_inputs[0][0] = 1 << 0; // Start consumes p0
        transition_outputs[0][0] = 1 << 1; // Start produces p1

        transition_inputs[1][0] = 1 << 1; // Normal consumes p1
        transition_outputs[1][0] = 1 << 2; // Normal produces p2

        transition_inputs[2][0] = 1 << 1; // Bypass consumes p1
        transition_outputs[2][0] = 1 << 2; // Bypass produces p2

        transition_inputs[3][0] = 1 << 2; // End consumes p2
        transition_outputs[3][0] = 1 << 3; // End produces p3

        Self {
            marking: SwarMarking::new(1), // Initial marking: 1 token in p0
            sketch: CountMinSketch::new(1024, 4),
            bandit: LinUcb::new(0.1),
            oc_dfg: StreamingOcDfg::new(),
            powl_model,
            config,
            state: AutonomicState {
                process_health: 1.0,
                throughput: 0.0,
                conformance_score: 1.0,
                drift_detected: false,
                active_cases: 0,
                field_elevation: 0.0,
                pack_posture: PackPosture::Nominal,
            },
            activity_table,
            transition_inputs,
            transition_outputs,
            trace_buffer: [0; 256],
            trace_cursor: 0,
            powl_executed_mask: KBitSet::zero(),
            powl_prev_idx: PowlModel::<WORDS>::MAX_NODES,
            last_context: std::cell::Cell::new([0.0; CONTEXT_DIM]),
            pre_action_conformance: 0.0,
            adaptation_count: 0,
            total_firings: 0,
            total_violations: 0,
            prediction_log: PredictionLogBuffer::<64>::new(1),
        }
    }

    fn state_context(&self, state: &AutonomicState) -> [f32; CONTEXT_DIM] {
        let mut ctx = [0.0f32; CONTEXT_DIM];
        ctx[0] = state.process_health;
        ctx[1] = state.conformance_score;
        ctx[2] = state.throughput / (1.0 + state.throughput);
        ctx[3] = state.drift_detected as u8 as f32;
        ctx[4] = (state.active_cases as f32 / 1000.0).min(1.0);
        let signal_keys = [
            "repair_signal",
            "opt_signal",
            "drift_signal",
            "escalate_signal",
            "health_signal",
        ];
        for (i, key) in signal_keys.iter().enumerate() {
            ctx[5 + i] = (self.sketch.estimate(key) as f32 / 100.0).clamp(0.0, 1.0);
        }
        ctx
    }

    /// Compute adaptive OCPM divergence threshold from binding frequency distribution.
    ///
    /// Scans binding_frequencies[0..4096] and computes mean + 2*std_dev to detect
    /// anomalous edge frequencies. Uses integer arithmetic throughout to avoid floats.
    /// Floor at OCPM_DIVERGENCE_THRESHOLD for cold-start safety.
    fn ocpm_divergence_threshold(&self) -> u32 {
        const EDGE_CACHE: usize = 4096;
        let mut sum: u64 = 0;
        let mut sum_sq: u128 = 0;

        // Accumulate frequency statistics
        for &freq in &self.oc_dfg.binding_frequencies[0..EDGE_CACHE] {
            sum = sum.wrapping_add(freq as u64);
            sum_sq = sum_sq.wrapping_add((freq as u128).wrapping_mul(freq as u128));
        }

        // Compute mean and variance using integer arithmetic
        let mean: u64 = sum / (EDGE_CACHE as u64);
        let mean_sq: u128 = (mean as u128).wrapping_mul(mean as u128);
        let mean_of_sq: u128 = sum_sq / (EDGE_CACHE as u128);
        let variance: u128 = if mean_of_sq > mean_sq {
            mean_of_sq - mean_sq
        } else {
            0
        };

        // Integer square root of variance for std_dev
        let std_dev = {
            if variance == 0 {
                0u64
            } else {
                // Newton-Raphson for u64
                let mut x = (variance as u64).wrapping_add(1) >> 1;
                let mut prev = 0u64;
                while x != prev && x > 0 {
                    prev = x;
                    let q = variance / (x as u128);
                    x = ((x as u128).wrapping_add(q)) as u64 >> 1;
                }
                x
            }
        };

        // Threshold = mean + 2*std_dev, floor at OCPM_DIVERGENCE_THRESHOLD
        let adaptive_threshold = (mean.wrapping_add(2u64.wrapping_mul(std_dev)))
            .max(OCPM_DIVERGENCE_THRESHOLD as u64) as u32;
        adaptive_threshold
    }
}

impl<const WORDS: usize> AutonomicKernel for Vision2030Kernel<WORDS> {
    fn observe(&mut self, event: AutonomicEvent) {
        self.sketch.add(&event.payload);

        let p = event.payload.to_lowercase();
        let act_idx_opt = if p.contains("start") {
            Some(0u8)
        } else if p.contains("normal") || p.contains("matched") {
            Some(1u8)
        } else if p.contains("bypass") || p.contains("skip") || p.contains("violation") {
            Some(2u8)
        } else if p.contains("end") || p.contains("limit") || p.contains("finish") {
            Some(3u8)
        } else {
            None
        };

        let activity_hash = fnv1a_64(event.payload.as_bytes());

        // Extract mock objects from payload for OCPM 2.0
        let mut mock_objects = [(0u64, 0u64, 0u64); 16];
        let mut mock_objects_len = 0;
        let qualifier_hash = if p.contains("creates") {
            QUALIFIER_CREATES
        } else if p.contains("updates") {
            QUALIFIER_UPDATES
        } else {
            QUALIFIER_READS
        };

        if p.contains("obj") || p.contains("order") || p.contains("item") {
            let id_hash = fnv1a_64(event.source.as_bytes());
            let type_hash = if p.contains("item") {
                ITEM_TYPE_HASH
            } else {
                ORDER_TYPE_HASH
            };
            if mock_objects_len < 16 {
                mock_objects[mock_objects_len] = (id_hash, type_hash, qualifier_hash);
                mock_objects_len += 1;
            }

            // Artificial divergence trigger
            if p.contains("divergence") {
                for i in 0..10 {
                    if mock_objects_len < 16 {
                        mock_objects[mock_objects_len] = (id_hash + i, type_hash, qualifier_hash);
                        mock_objects_len += 1;
                    }
                }
            }
        } else if mock_objects_len < 16 {
            mock_objects[mock_objects_len] =
                (fnv1a_64(event.source.as_bytes()), 0x0, qualifier_hash);
            mock_objects_len += 1;
        }

        self.oc_dfg
            .observe_event(activity_hash, &mock_objects[..mock_objects_len]);

        if p.contains("relates to") || p.contains("belongs to") {
            self.oc_dfg
                .observe_o2o(ORDER_TYPE_HASH, ITEM_TYPE_HASH, fnv1a_64(b"contains"));
        }

        // Check for OCPM binding anomaly
        let mut ocpm_drift = false;

        if p.contains("changed") || p.contains("value") {
            self.oc_dfg
                .observe_object_change(fnv1a_64(event.source.as_bytes()), fnv1a_64(b"amount"));
            if p.contains("critical") {
                ocpm_drift = true;
            }
        }

        let adaptive_threshold = self.ocpm_divergence_threshold();
        for &(_id_hash, type_hash, qual_hash) in &mock_objects[..mock_objects_len] {
            let binding_hash = activity_hash
                .wrapping_mul(FNV_MIX_PRIME)
                .wrapping_add(type_hash)
                .wrapping_mul(FNV_MIX_PRIME)
                .wrapping_add(qual_hash);
            let binding_idx = (binding_hash as usize) & EDGE_MASK_4096;
            if self.oc_dfg.binding_frequencies[binding_idx] > adaptive_threshold {
                ocpm_drift = true;
            }
        }

        // Apply OCPM drift regardless of whether activity is matched
        self.state.drift_detected =
            select_u64(ocpm_drift as u64, 1, self.state.drift_detected as u64) != 0;
        if ocpm_drift {
            self.state.conformance_score =
                (self.state.conformance_score - CONFORMANCE_PENALTY_SWAR_VIOLATION).max(0.0);
            self.state.process_health =
                (self.state.process_health - HEALTH_PENALTY_SWAR_VIOLATION).max(0.0);

            // Emit Detection bark from detector breed
            let _bark = BarkEvent {
                kind: BarkKind::Detection,
                source_breed: "detector",
                signal_hash: fnv1a_64(b"ocpm_drift"),
                timestamp_us: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_micros() as u64)
                    .unwrap_or(0),
                tighten: true,
            };

            // Tighten pack posture on detection
            self.state.pack_posture = match self.state.pack_posture {
                PackPosture::Nominal => PackPosture::Elevated,
                PackPosture::Elevated => PackPosture::Tightened,
                PackPosture::Tightened => PackPosture::Lockdown,
                PackPosture::Lockdown => PackPosture::Lockdown,
            };
        }

        if let Some(idx) = act_idx_opt {
            if self.trace_cursor < 256 {
                self.trace_buffer[self.trace_cursor] = idx;
                self.trace_cursor += 1;
            } else {
                // Circular buffer
                self.trace_buffer.rotate_left(1);
                self.trace_buffer[255] = idx;
            }

            // 1. Incremental Semantic Check (POWL) - BCINR Optimization O(1)
            let is_valid = self.powl_model.is_transition_valid(
                idx as usize,
                self.powl_executed_mask,
                self.powl_prev_idx,
            );

            if !is_valid {
                self.state.process_health =
                    (self.state.process_health - HEALTH_PENALTY_POWL_VIOLATION).max(0.0);
            }

            // Update execution state branchlessly
            let _ = self.powl_executed_mask.set(idx as usize);
            self.powl_prev_idx = idx as usize;

            // Branchless status updates using BCINR select_u64
            self.state.drift_detected =
                select_u64(!is_valid as u64, 1, self.state.drift_detected as u64) != 0;

            // 2. Token Replay (SWAR)
            let req = &self.transition_inputs[idx as usize];
            let out = &self.transition_outputs[idx as usize];
            let (new_marking, fired) = self.marking.try_fire_branchless(req, out);
            self.marking = new_marking;

            // Update conformance score branchlessly-ish
            if !fired {
                self.state.conformance_score =
                    (self.state.conformance_score - CONFORMANCE_PENALTY_SWAR_VIOLATION).max(0.0);
                self.state.process_health =
                    (self.state.process_health - HEALTH_PENALTY_SWAR_VIOLATION).max(0.0);
            }

            // Phase 2 State tracking: active cases
            self.state.active_cases = self.sketch.estimate(&event.payload) as usize;
        }

        self.state.throughput += 1.0;
    }

    fn infer(&self) -> AutonomicState {
        self.state.clone()
    }

    fn propose(&self, state: &AutonomicState) -> Vec<AutonomicAction> {
        const ACTION_PALETTE: [(ActionType, ActionRisk, &str, u64); 4] = [
            (ActionType::Recommend, ActionRisk::Low, "Throughput optimization", 101),
            (ActionType::Repair, ActionRisk::Medium, "Axiomatic structural repair", 102),
            (ActionType::Escalate, ActionRisk::High, "Critical escalation", 103),
            (ActionType::Recover, ActionRisk::Low, "Evidence recovery from audit log", 104),
        ];

        let context = self.state_context(state);
        self.last_context.set(context);
        let action_idx = self.bandit.select_action(&context, 4);

        // Heuristic q-values — ties (both 0.0 at cold start) resolve to Repair when drifted
        let q_repair =
            state.conformance_score / (1.0 + self.sketch.estimate("repair_signal") as f32);
        let q_opt =
            state.process_health / (1.0 + self.sketch.estimate("opt_signal") as f32);

        let candidates = if state.drift_detected && q_repair >= q_opt {
            vec![
                AutonomicAction::new(
                    ACTION_PALETTE[1].3,
                    ACTION_PALETTE[1].0,
                    ACTION_PALETTE[1].1,
                    ACTION_PALETTE[1].2,
                ),
                AutonomicAction::new(
                    ACTION_PALETTE[2].3,
                    ACTION_PALETTE[2].0,
                    ACTION_PALETTE[2].1,
                    ACTION_PALETTE[2].2,
                ),
            ]
        } else {
            let arm = action_idx.min(3);
            vec![AutonomicAction::new(
                ACTION_PALETTE[arm].3,
                ACTION_PALETTE[arm].0,
                ACTION_PALETTE[arm].1,
                ACTION_PALETTE[arm].2,
            )]
        };

        // MDF wiring: if MDF finds a minimum-decisive action, return it; else fall through to candidates
        use crate::autonomic::MinimumDecisiveForce;
        if let Some(mdf_action) = MinimumDecisiveForce::is_minimal_decisive(&candidates, state) {
            return vec![mdf_action.clone()];
        }

        candidates
    }

    fn accept(&self, action: &AutonomicAction, state: &AutonomicState) -> bool {
        let sim = Simulator::new(state.clone());
        let (_, expected_reward) = sim.evaluate_action(action);

        if action.risk_profile >= ActionRisk::High {
            let accepted = expected_reward > 0.0;
            return accepted;
        }

        let threshold = match self.config.autonomic.guards.risk_threshold.as_str() {
            "Low" => ActionRisk::Low,
            "Medium" => ActionRisk::Medium,
            "High" => ActionRisk::High,
            _ => ActionRisk::Critical,
        };

        action.risk_profile <= threshold
    }

    fn execute(&mut self, action: AutonomicAction) -> AutonomicResult {
        let t_start = std::time::Instant::now();

        let is_repair = (action.action_type == ActionType::Repair) as u64;
        let is_recover = (action.action_type == ActionType::Recover) as u64;

        self.pre_action_conformance = self.state.conformance_score;
        self.total_firings = self.total_firings.saturating_add(1);

        self.state.drift_detected = select_u64(is_repair | is_recover, 0, self.state.drift_detected as u64) != 0;

        if is_repair != 0 {
            self.trace_cursor = 0;
            self.state.conformance_score =
                (self.state.conformance_score + CONFORMANCE_REWARD_REPAIR).min(1.0);
            self.state.process_health = (self.state.process_health + HEALTH_REWARD_REPAIR).min(1.0);
        } else if is_recover != 0 {
            // Recover action: attempt to pull last positive entry from prediction log
            if let Some(_entry) = self.prediction_log.last_positive_entry() {
                self.state.conformance_score =
                    (self.state.conformance_score + CONFORMANCE_REWARD_RECOVER).min(1.0);
                self.state.process_health = (self.state.process_health + HEALTH_REWARD_RECOVER).min(1.0);
            }
        }

        let execution_latency_ms = t_start.elapsed().as_millis() as u64;
        let manifest_hash = fnv1a_64(action.parameters.as_bytes());

        AutonomicResult {
            success: true,
            execution_latency_ms,
            manifest_hash,
        }
    }

    fn manifest(&self, result: &AutonomicResult) -> String {
        format!(
            "VISION_2030_MANIFEST: success={}, hash={:X}, marking={:X}",
            result.success, result.manifest_hash, self.marking.words[0]
        )
    }

    fn adapt(&mut self, feedback: AutonomicFeedback) {
        let delta = self.state.conformance_score - self.pre_action_conformance;
        let derived_reward = if feedback.human_override {
            -1.0f32
        } else {
            delta.clamp(-1.0, 1.0)
        };

        let bandit_ctx = self.last_context.get();
        self.bandit.update(&bandit_ctx, derived_reward);
        self.adaptation_count = self.adaptation_count.saturating_add(1);

        let decay = if feedback.reward < 0.0 {
            HEALTH_DECAY_NEGATIVE_REWARD
        } else {
            -HEALTH_IMPROVEMENT_POSITIVE_REWARD
        };
        self.state.process_health = (self.state.process_health - decay).clamp(0.0, 1.0);
    }
}
