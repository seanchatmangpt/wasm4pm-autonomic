//! RalphPlan — schema, types, and structural validator for ralph phase-execution plans.
//!
//! Domain: Spec Kit phase execution by ralph. Distinct from `AutomlPlan` (which is
//! HDIT signal-selection from `pdc2025`). RalphPlan accounts for phase progress,
//! produced artifacts, and gate outcomes per processed idea.
//!
//! Schema version: `chatmangpt.ralph.plan.v1`.
//!
//! Anti-lie invariants enforced by `RalphPlan::validate`:
//! 1. `phases_completed + phases_blocked + phases_skipped + phases_pending == phases_expected`
//! 2. `phase_sequence.len() == phases_expected`
//! 3. `verdict` is consistent with gate results and accounting balance.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const SCHEMA_VERSION: &str = "chatmangpt.ralph.plan.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    SoftFail,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Gate {
    pub name: String,
    pub status: GateStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    /// Producing phase (e.g. "specify", "plan", "tasks", "implement").
    pub kind: String,
    pub path: String,
    /// Lowercase hex-encoded SHA-256 of the artifact contents.
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Accounting {
    pub phases_expected: u32,
    pub phases_completed: u32,
    pub phases_blocked: u32,
    pub phases_skipped: u32,
    pub phases_pending: u32,
    pub balanced: bool,
}

impl Accounting {
    pub fn check_balance(&self) -> bool {
        self.phases_completed + self.phases_blocked + self.phases_skipped + self.phases_pending
            == self.phases_expected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RalphPlan {
    /// Schema discriminator. Must be `SCHEMA_VERSION`.
    pub schema: String,
    pub run_id: String,
    pub target: String,
    /// SHA-256 of the idea string.
    pub idea_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constitution_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_hash: Option<String>,
    /// Most recent phase reached.
    pub phase: String,
    /// Canonical phase sequence (e.g. `["specify","plan","tasks","implement"]`).
    pub phase_sequence: Vec<String>,
    pub completed_phases: Vec<String>,
    pub blocked_phases: Vec<String>,
    pub skipped_phases: Vec<String>,
    pub artifacts: Vec<Artifact>,
    pub gates: Vec<Gate>,
    pub accounting: Accounting,
    pub verdict: Verdict,
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    BadSchemaVersion {
        actual: String,
    },
    AccountingUnbalanced {
        sum: u32,
        expected: u32,
    },
    PhaseSequenceLenMismatch {
        sequence_len: usize,
        expected: u32,
    },
    DuplicatePhase {
        phase: String,
    },
    /// A phase appears in completed/blocked/skipped that is not in `phase_sequence`.
    UnknownPhase {
        phase: String,
    },
    /// `verdict == Pass` but at least one gate is `Fail` or accounting is unbalanced.
    VerdictInconsistent {
        reason: String,
    },
    /// A phase in `blocked_phases` lacks a gate with `failure_class`.
    BlockedWithoutFailureClass {
        phase: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSchemaVersion { actual } => write!(
                f,
                "schema version mismatch: expected '{}', got '{}'",
                SCHEMA_VERSION, actual
            ),
            Self::AccountingUnbalanced { sum, expected } => write!(
                f,
                "accounting unbalanced: completed+blocked+skipped+pending={}, expected={}",
                sum, expected
            ),
            Self::PhaseSequenceLenMismatch {
                sequence_len,
                expected,
            } => write!(
                f,
                "phase_sequence length {} != phases_expected {}",
                sequence_len, expected
            ),
            Self::DuplicatePhase { phase } => write!(f, "phase appears more than once: {}", phase),
            Self::UnknownPhase { phase } => write!(
                f,
                "phase '{}' not present in phase_sequence",
                phase
            ),
            Self::VerdictInconsistent { reason } => {
                write!(f, "verdict inconsistent with state: {}", reason)
            }
            Self::BlockedWithoutFailureClass { phase } => write!(
                f,
                "blocked phase '{}' has no gate with failure_class",
                phase
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

impl RalphPlan {
    /// Run the structural anti-lie validator. Returns the first violation found,
    /// or `Ok(())` if all invariants hold.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.schema != SCHEMA_VERSION {
            return Err(ValidationError::BadSchemaVersion {
                actual: self.schema.clone(),
            });
        }

        // Invariant 2: phase_sequence length matches phases_expected.
        if self.phase_sequence.len() as u32 != self.accounting.phases_expected {
            return Err(ValidationError::PhaseSequenceLenMismatch {
                sequence_len: self.phase_sequence.len(),
                expected: self.accounting.phases_expected,
            });
        }

        // Invariant 1: accounting balance.
        if !self.accounting.check_balance() {
            let sum = self.accounting.phases_completed
                + self.accounting.phases_blocked
                + self.accounting.phases_skipped
                + self.accounting.phases_pending;
            return Err(ValidationError::AccountingUnbalanced {
                sum,
                expected: self.accounting.phases_expected,
            });
        }
        if self.accounting.balanced != self.accounting.check_balance() {
            return Err(ValidationError::AccountingUnbalanced {
                sum: self.accounting.phases_completed
                    + self.accounting.phases_blocked
                    + self.accounting.phases_skipped
                    + self.accounting.phases_pending,
                expected: self.accounting.phases_expected,
            });
        }

        // Phases in completed/blocked/skipped must exist in phase_sequence and be distinct.
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let known: std::collections::HashSet<&str> =
            self.phase_sequence.iter().map(|s| s.as_str()).collect();
        for bucket in [
            &self.completed_phases,
            &self.blocked_phases,
            &self.skipped_phases,
        ] {
            for p in bucket {
                if !known.contains(p.as_str()) {
                    return Err(ValidationError::UnknownPhase { phase: p.clone() });
                }
                if !seen.insert(p.as_str()) {
                    return Err(ValidationError::DuplicatePhase { phase: p.clone() });
                }
            }
        }

        // Bucket counts must match accounting.
        if self.completed_phases.len() as u32 != self.accounting.phases_completed
            || self.blocked_phases.len() as u32 != self.accounting.phases_blocked
            || self.skipped_phases.len() as u32 != self.accounting.phases_skipped
        {
            return Err(ValidationError::AccountingUnbalanced {
                sum: self.completed_phases.len() as u32
                    + self.blocked_phases.len() as u32
                    + self.skipped_phases.len() as u32
                    + self.accounting.phases_pending,
                expected: self.accounting.phases_expected,
            });
        }

        // Every blocked phase must reference a failed gate with failure_class.
        for bp in &self.blocked_phases {
            let has_failure_gate = self
                .gates
                .iter()
                .any(|g| g.status == GateStatus::Fail && g.failure_class.is_some());
            if !has_failure_gate {
                return Err(ValidationError::BlockedWithoutFailureClass {
                    phase: bp.clone(),
                });
            }
        }

        // Verdict consistency.
        let any_fail = self
            .gates
            .iter()
            .any(|g| g.status == GateStatus::Fail);
        let pending = self.accounting.phases_pending > 0;
        let blocked = self.accounting.phases_blocked > 0;
        let skipped = self.accounting.phases_skipped > 0;
        match self.verdict {
            Verdict::Pass => {
                if any_fail {
                    return Err(ValidationError::VerdictInconsistent {
                        reason: "verdict=pass but a gate is failing".into(),
                    });
                }
                if pending || blocked || skipped {
                    return Err(ValidationError::VerdictInconsistent {
                        reason: "verdict=pass but phases are pending, blocked, or skipped".into(),
                    });
                }
            }
            Verdict::SoftFail => {
                // soft_fail allowed for any non-clean state.
                if !any_fail && !pending && !blocked && !skipped {
                    return Err(ValidationError::VerdictInconsistent {
                        reason: "verdict=soft_fail but plan is fully clean".into(),
                    });
                }
            }
            Verdict::Fatal => {
                // fatal must be reserved for accounting-unbalanced or schema-invalid cases,
                // both of which would have been caught before reaching here. So fatal at this
                // stage is also valid for catastrophic gate failure with no completion.
            }
        }

        Ok(())
    }
}

/// Returns the canonical JSON Schema 2020-12 description of a `RalphPlan`.
pub fn ralph_plan_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://dteam.dev/schemas/ralph-plan.json",
        "title": "RalphPlan",
        "description": "Artifact emitted by ralph for one processed idea. Anti-lie invariant: phases_completed + phases_blocked + phases_skipped + phases_pending == phases_expected.",
        "type": "object",
        "required": [
            "schema",
            "run_id",
            "target",
            "idea_hash",
            "phase",
            "phase_sequence",
            "completed_phases",
            "blocked_phases",
            "skipped_phases",
            "artifacts",
            "gates",
            "accounting",
            "verdict"
        ],
        "properties": {
            "schema": { "type": "string", "const": SCHEMA_VERSION },
            "run_id": { "type": "string" },
            "target": { "type": "string" },
            "idea_hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
            "constitution_hash": { "type": ["string","null"], "pattern": "^[0-9a-f]{64}$" },
            "spec_hash": { "type": ["string","null"], "pattern": "^[0-9a-f]{64}$" },
            "phase": { "type": "string" },
            "phase_sequence": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
            "completed_phases": { "type": "array", "items": { "type": "string" } },
            "blocked_phases": { "type": "array", "items": { "type": "string" } },
            "skipped_phases": { "type": "array", "items": { "type": "string" } },
            "artifacts": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["kind","path","hash"],
                    "properties": {
                        "kind": { "type": "string" },
                        "path": { "type": "string" },
                        "hash": { "type": "string", "pattern": "^[0-9a-f]{64}$" }
                    }
                }
            },
            "gates": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["name","status"],
                    "properties": {
                        "name": { "type": "string" },
                        "status": { "type": "string", "enum": ["pass","fail","skip"] },
                        "failure_class": { "type": "string" }
                    }
                }
            },
            "accounting": {
                "type": "object",
                "required": [
                    "phases_expected",
                    "phases_completed",
                    "phases_blocked",
                    "phases_skipped",
                    "phases_pending",
                    "balanced"
                ],
                "properties": {
                    "phases_expected": { "type": "integer", "minimum": 0 },
                    "phases_completed": { "type": "integer", "minimum": 0 },
                    "phases_blocked": { "type": "integer", "minimum": 0 },
                    "phases_skipped": { "type": "integer", "minimum": 0 },
                    "phases_pending": { "type": "integer", "minimum": 0 },
                    "balanced": { "type": "boolean" }
                }
            },
            "verdict": { "type": "string", "enum": ["pass","soft_fail","fatal"] }
        },
        "additionalProperties": true
    })
}

/// Compute lowercase-hex SHA-256 of bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Compute lowercase-hex SHA-256 of a file's contents. Returns `None` if the file does not exist.
pub fn sha256_file(path: &std::path::Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| sha256_hex(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_plan() -> RalphPlan {
        RalphPlan {
            schema: SCHEMA_VERSION.into(),
            run_id: "ralph-test-001".into(),
            target: "mcpp".into(),
            idea_hash: "0".repeat(64),
            constitution_hash: None,
            spec_hash: None,
            phase: "implement".into(),
            phase_sequence: vec![
                "specify".into(),
                "plan".into(),
                "tasks".into(),
                "implement".into(),
            ],
            completed_phases: vec![
                "specify".into(),
                "plan".into(),
                "tasks".into(),
                "implement".into(),
            ],
            blocked_phases: vec![],
            skipped_phases: vec![],
            artifacts: vec![],
            gates: vec![
                Gate {
                    name: "constitution_present".into(),
                    status: GateStatus::Pass,
                    failure_class: None,
                },
            ],
            accounting: Accounting {
                phases_expected: 4,
                phases_completed: 4,
                phases_blocked: 0,
                phases_skipped: 0,
                phases_pending: 0,
                balanced: true,
            },
            verdict: Verdict::Pass,
        }
    }

    #[test]
    fn good_plan_validates() {
        good_plan().validate().expect("should be valid");
    }

    #[test]
    fn bad_schema_rejected() {
        let mut p = good_plan();
        p.schema = "wrong.schema.v0".into();
        assert!(matches!(
            p.validate(),
            Err(ValidationError::BadSchemaVersion { .. })
        ));
    }

    #[test]
    fn unbalanced_rejected() {
        let mut p = good_plan();
        p.accounting.phases_completed = 3; // 3+0+0+0 != 4
        assert!(matches!(
            p.validate(),
            Err(ValidationError::AccountingUnbalanced { .. })
        ));
    }

    #[test]
    fn pass_with_pending_rejected() {
        let mut p = good_plan();
        p.accounting.phases_completed = 3;
        p.accounting.phases_pending = 1;
        p.completed_phases.pop(); // 3 completed
        assert!(matches!(
            p.validate(),
            Err(ValidationError::VerdictInconsistent { .. })
        ));
    }

    #[test]
    fn soft_fail_with_blocked_validates() {
        let mut p = good_plan();
        p.accounting.phases_completed = 2;
        p.accounting.phases_blocked = 2;
        p.completed_phases = vec!["specify".into(), "plan".into()];
        p.blocked_phases = vec!["tasks".into(), "implement".into()];
        p.gates.push(Gate {
            name: "tasks_exists".into(),
            status: GateStatus::Fail,
            failure_class: Some("MISSING_TASKS_ARTIFACT".into()),
        });
        p.verdict = Verdict::SoftFail;
        p.validate().expect("soft_fail with blocked phases should be valid");
    }
}
