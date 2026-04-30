//! Strict serde shape for LLM-proposed OCEL worlds.
//!
//! Every field is required unless explicitly `#[serde(default)]`. The
//! `expectedResponse` / `response` fields reuse [`ccog::AutonomicInstinct`]
//! so a non-canonical class fails at the type level.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ccog::instinct::AutonomicInstinct;

/// LLM-emitted world. Mirrors the prompt contract verbatim.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OcelWorld {
    /// AutoInstinct version the prompt targeted.
    pub version: String,
    /// Pack profile (e.g. `supply-chain`).
    pub profile: String,
    /// Scenario name (e.g. `dock-obstruction-cold-chain`).
    pub scenario: String,
    /// Object instances with public-ontology types.
    pub objects: Vec<OcelObject>,
    /// Events linking objects with timestamps.
    pub events: Vec<OcelEvent>,
    /// Counterfactual perturbations the gauntlet must prove against.
    pub counterfactuals: Vec<Counterfactual>,
    /// Expected response classes per condition (positive triad).
    pub expected_instincts: Vec<ExpectedInstinct>,
}

/// One OCEL object.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OcelObject {
    /// Object id (referenced by events).
    pub id: String,
    /// Discriminator within the world (free text, lowercased).
    #[serde(rename = "type")]
    pub kind: String,
    /// Human-readable label.
    pub label: String,
    /// Public-ontology type IRI (must satisfy
    /// [`crate::doctrine::public_ontology_profiles`]).
    pub ontology_type: String,
    /// Free attribute bag.
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

/// One OCEL event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OcelEvent {
    /// Event id.
    pub id: String,
    /// Event discriminator.
    #[serde(rename = "type")]
    pub kind: String,
    /// ISO-8601 timestamp string.
    pub time: String,
    /// Public-ontology event type.
    pub ontology_type: String,
    /// Object ids referenced by this event.
    pub objects: Vec<String>,
    /// Free attribute bag.
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

/// One counterfactual perturbation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Counterfactual {
    /// Counterfactual id.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Object ids to remove before re-running the scenario.
    #[serde(default)]
    pub remove_objects: Vec<String>,
    /// Event ids to remove before re-running the scenario.
    #[serde(default)]
    pub remove_events: Vec<String>,
    /// Canonical response the gauntlet must observe under this perturbation.
    pub expected_response: AutonomicInstinct,
}

/// Positive expectation: under `condition`, AutoInstinct must respond with
/// `response` and must never fall into `forbidden`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedInstinct {
    /// Free-form condition (interpreted by the JTBD generator).
    pub condition: String,
    /// Canonical response class.
    pub response: AutonomicInstinct,
    /// Forbidden behaviors (e.g. `"fake-completion"`).
    #[serde(default)]
    pub forbidden: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_field_is_rejected() {
        let json = r#"{
            "version":"30.1.1","profile":"x","scenario":"y",
            "objects":[],"events":[],"counterfactuals":[],"expectedInstincts":[],
            "extraJunk": true
        }"#;
        assert!(serde_json::from_str::<OcelWorld>(json).is_err());
    }

    #[test]
    fn response_outside_lattice_fails_serde() {
        let json = r#"{
            "version":"30.1.1","profile":"x","scenario":"y",
            "objects":[],"events":[],
            "counterfactuals":[{
                "id":"c1","description":"d",
                "removeObjects":[],"removeEvents":[],
                "expectedResponse":"Bark"
            }],
            "expectedInstincts":[]
        }"#;
        assert!(serde_json::from_str::<OcelWorld>(json).is_err());
    }

    #[test]
    fn canonical_response_parses() {
        let json = r#"{
            "version":"30.1.1","profile":"supply-chain","scenario":"dock",
            "objects":[],"events":[],
            "counterfactuals":[{
                "id":"c1","description":"d",
                "removeObjects":[],"removeEvents":[],
                "expectedResponse":"Settle"
            }],
            "expectedInstincts":[{
                "condition":"badge+assignment","response":"Settle","forbidden":[]
            }]
        }"#;
        let w: OcelWorld = serde_json::from_str(json).unwrap();
        assert_eq!(w.counterfactuals[0].expected_response, AutonomicInstinct::Settle);
        assert_eq!(w.expected_instincts[0].response, AutonomicInstinct::Settle);
    }
}
