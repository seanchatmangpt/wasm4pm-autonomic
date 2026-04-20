use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicEvent {
    pub source: String,
    pub payload: String,
    pub timestamp: SystemTime,
}

impl fmt::Display for AutonomicEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Event] {} (source: {})", self.payload, self.source)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicState {
    pub process_health: f32,
    pub throughput: f32,
    pub conformance_score: f32,
    pub drift_detected: bool,
    pub active_cases: usize,
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
            "⚠️ DRIFT DETECTED"
        } else {
            "✅ STABLE"
        };
        write!(
            f,
            "{} Health: {:.1}% | Throughput: {:.2} eps | Conf: {:.2} | Cases: {} | {}",
            health_emoji,
            self.process_health * 100.0,
            self.throughput,
            self.conformance_score,
            self.active_cases,
            drift_str
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

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = match self {
            ActionType::Recommend => "Recommend",
            ActionType::Approve => "Approve",
            ActionType::Reject => "Reject",
            ActionType::Escalate => "Escalate",
            ActionType::Pause => "Pause",
            ActionType::Retry => "Retry",
            ActionType::Reroute => "Reroute",
            ActionType::Repair => "Repair",
            ActionType::Notify => "Notify",
        };
        write!(f, "{}", type_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicAction {
    pub action_id: u64,
    pub action_type: ActionType,
    pub risk_profile: ActionRisk,
    pub parameters: String,
    pub required_authority: String,
}

impl fmt::Display for AutonomicAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "⚙️  Action #{} [{}]: {} (Risk: {})",
            self.action_id, self.action_type, self.parameters, self.risk_profile
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicResult {
    pub success: bool,
    pub execution_latency_ms: u64,
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
            "{} | Latency: {}ms | Hash: {:X}",
            status, self.execution_latency_ms, self.manifest_hash
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicFeedback {
    pub reward: f32,
    pub human_override: bool,
    pub side_effects: Vec<String>,
}

impl AutonomicAction {
    pub fn new(id: u64, action_type: ActionType, risk: ActionRisk, params: &str) -> Self {
        Self {
            action_id: id,
            action_type,
            risk_profile: risk,
            parameters: params.to_string(),
            required_authority: "system".to_string(),
        }
    }

    pub fn recommend(id: u64, params: &str) -> Self {
        Self::new(id, ActionType::Recommend, ActionRisk::Low, params)
    }

    pub fn critical(id: u64, action_type: ActionType, params: &str) -> Self {
        Self::new(id, action_type, ActionRisk::Critical, params)
    }
}
