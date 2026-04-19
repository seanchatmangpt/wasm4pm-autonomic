pub mod models;
pub mod conformance;
pub mod io;
pub mod utils;
pub mod reinforcement;
pub mod reinforcement_tests;

// Re-export models for easier access
pub use models::*;
pub use conformance::*;

// Dummy structures needed for trait implementations in tests
#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub struct RlState {
    pub health_level: i32,
    pub event_rate_q: i32,
    pub activity_count_q: i32,
    pub spc_alert_level: i32,
    pub drift_status: i32,
    pub rework_ratio_q: i32,
    pub circuit_state: i32,
    pub cycle_phase: i32,
    pub marking_vec: Vec<(String, usize)>,
    pub recent_activities: Vec<String>,
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
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
impl reinforcement::WorkflowState for RlState {
    fn features(&self) -> Vec<f32> { vec![self.health_level as f32] }
    fn is_terminal(&self) -> bool { self.health_level < 0 || self.health_level >= 5 }
}

pub mod rl_state_serialization {
    use std::collections::HashMap;
    pub struct SerializedAgentQTable {
        pub agent_type: u8,
        pub state_values: HashMap<i64, Vec<f32>>,
    }
    pub fn encode_rl_state_key(h: i32, _e: i32, _a: i32, _s: i32, _d: i32, _r: i32, _c: i32, _p: i32) -> i64 { h as i64 }
    pub fn decode_rl_state_key(key: i64) -> (i32, i32, i32, i32, i32, i32, i32, i32) { (key as i32,0,0,0,0,0,0,0) }
}
pub mod automation;
pub mod benchmark;
pub mod config;
pub mod ref_models {
    pub mod ref_petri_net;
    pub mod ref_event_log;
}
pub mod ref_conformance {
    pub mod ref_token_replay;
}
