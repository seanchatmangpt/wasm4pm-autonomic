use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutonomicEvent {
    pub source_hash: u64,
    pub activity_idx: u8,
    pub payload_hash: u64,
    pub timestamp_ns: u64,
}

impl fmt::Display for AutonomicEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[Event] act:{} hash:{:X} (source:{:X})",
            self.activity_idx, self.payload_hash, self.source_hash
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutonomicState {
    pub process_health: f32,
    pub throughput: f32,
    pub conformance_score: f32,
    pub drift_detected: bool,
    pub drift_occurred: bool, // Sticky bit for current observation cycle
    pub active_cases: usize,
    pub control_surface_hash: u64,
}

impl fmt::Display for AutonomicState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let health_emoji = if self.process_health > 0.8 {
            "🟢"
        } else if self.process_health > 0.4 {
            "🟡"
        } else {
            "🔴"
        };
        let drift_str = if self.drift_detected {
            "⚠️ DRIFT"
        } else if self.drift_occurred {
            "🩹 REPAIRED"
        } else {
            "✅ STABLE"
        };
        write!(
            f,
            "{} Health: {:.1}% | Conf: {:.2} | {} | CS:{:X}",
            health_emoji,
            self.process_health * 100.0,
            self.conformance_score,
            drift_str,
            self.control_surface_hash
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ActionRisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let risk_str = match self {
            ActionRisk::Low => "LOW",
            ActionRisk::Medium => "MEDIUM",
            ActionRisk::High => "HIGH",
            ActionRisk::Critical => "CRITICAL",
        };
        write!(f, "{}", risk_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Recommend,
    Approve,
    Reject,
    Escalate,
    Pause,
    Retry,
    Reroute,
    Repair,
    Notify,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutonomicAction {
    pub action_id: u64,
    pub action_type: ActionType,
    pub risk_profile: ActionRisk,
    pub parameters_hash: u64,
}

impl fmt::Display for AutonomicAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "⚙️  Action #{} [{:?}] (Risk: {})",
            self.action_id, self.action_type, self.risk_profile
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutonomicResult {
    pub success: bool,
    pub execution_latency_ns: u64,
    pub manifest_hash: u64,
}

impl fmt::Display for AutonomicResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.success {
            "✅ SUCCESS"
        } else {
            "❌ FAILED"
        };
        write!(
            f,
            "{} | Latency: {}ns | Hash: {:X}",
            status, self.execution_latency_ns, self.manifest_hash
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutonomicFeedback {
    pub reward: f32,
    pub human_override: bool,
}

impl AutonomicAction {
    pub fn new(id: u64, action_type: ActionType, risk: ActionRisk, params_hash: u64) -> Self {
        Self {
            action_id: id,
            action_type,
            risk_profile: risk,
            parameters_hash: params_hash,
        }
    }
}
