//! Field-pack compilation.
//!
//! Once a candidate policy passes the gauntlet, [`compile`] emits a
//! deployable [`FieldPackArtifact`]: a serializable record containing the
//! policy's rules, ontology profile, admitted breeds, version, and
//! `urn:blake3` digest of the canonical bytes (so receipts can prove which
//! pack produced which decision).

use serde::{Deserialize, Serialize};

use crate::synth::CandidatePolicy;
use crate::AUTOINSTINCT_VERSION;

/// Serializable field-pack artifact.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldPackArtifact {
    /// Pack name (e.g. "lifestyle", "supply-chain").
    pub name: String,
    /// AutoInstinct compiler version.
    pub autoinstinct_version: String,
    /// Public ontology prefixes the pack admits.
    pub ontology_profile: Vec<String>,
    /// Admitted breed names (string ids — not enums to keep the artifact
    /// stable across `verdict::Breed` renumbering).
    pub admitted_breeds: Vec<String>,
    /// Rules in deterministic order.
    pub rules: Vec<(String, ccog::instinct::AutonomicInstinct)>,
    /// Default response.
    pub default_response: ccog::instinct::AutonomicInstinct,
    /// `urn:blake3:` of the canonical artifact bytes (excluding this field).
    pub digest_urn: String,
}

/// Inputs to compilation.
#[derive(Clone, Debug)]
pub struct CompileInputs<'a> {
    /// Pack name.
    pub name: &'a str,
    /// Public ontology prefixes the pack will be allowed to emit.
    pub ontology_profile: &'a [&'a str],
    /// String ids of admitted ccog breeds.
    pub admitted_breeds: &'a [&'a str],
    /// Gauntlet-admitted candidate policy.
    pub policy: &'a CandidatePolicy,
}

/// Compile a gauntlet-admitted policy into a deployable artifact.
#[must_use]
pub fn compile(inputs: CompileInputs<'_>) -> FieldPackArtifact {
    let mut artifact = FieldPackArtifact {
        name: inputs.name.to_string(),
        autoinstinct_version: AUTOINSTINCT_VERSION.to_string(),
        ontology_profile: inputs
            .ontology_profile
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        admitted_breeds: inputs
            .admitted_breeds
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        rules: inputs.policy.rules.clone(),
        default_response: inputs.policy.default,
        digest_urn: String::new(),
    };
    let canonical = canonical_bytes(&artifact);
    let digest = blake3::hash(&canonical);
    artifact.digest_urn = format!("urn:blake3:{}", digest.to_hex());
    artifact
}

/// Canonical bytes used to compute the digest. Stable across runs:
/// JSON with `digest_urn` cleared so the digest never references itself.
fn canonical_bytes(artifact: &FieldPackArtifact) -> Vec<u8> {
    let mut a = artifact.clone();
    a.digest_urn.clear();
    serde_json::to_vec(&a).expect("FieldPackArtifact is always serializable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AutonomicInstinct;

    fn sample_policy() -> CandidatePolicy {
        CandidatePolicy {
            rules: vec![("urn:blake3:a".into(), AutonomicInstinct::Ask)],
            default: AutonomicInstinct::Ignore,
        }
    }

    #[test]
    fn compile_produces_urn_blake3_digest() {
        let policy = sample_policy();
        let pack = compile(CompileInputs {
            name: "test-pack",
            ontology_profile: &["http://www.w3.org/ns/prov#"],
            admitted_breeds: &["eliza", "mycin"],
            policy: &policy,
        });
        assert!(pack.digest_urn.starts_with("urn:blake3:"));
        assert_eq!(pack.autoinstinct_version, AUTOINSTINCT_VERSION);
    }

    #[test]
    fn compile_is_deterministic() {
        let policy = sample_policy();
        let p1 = compile(CompileInputs {
            name: "test-pack",
            ontology_profile: &["http://www.w3.org/ns/prov#"],
            admitted_breeds: &["eliza"],
            policy: &policy,
        });
        let p2 = compile(CompileInputs {
            name: "test-pack",
            ontology_profile: &["http://www.w3.org/ns/prov#"],
            admitted_breeds: &["eliza"],
            policy: &policy,
        });
        assert_eq!(p1, p2);
    }

    #[test]
    fn compile_digest_changes_with_rules() {
        let mut policy = sample_policy();
        let p1 = compile(CompileInputs {
            name: "test-pack",
            ontology_profile: &[],
            admitted_breeds: &[],
            policy: &policy,
        });
        policy.rules.push(("urn:blake3:b".into(), AutonomicInstinct::Inspect));
        let p2 = compile(CompileInputs {
            name: "test-pack",
            ontology_profile: &[],
            admitted_breeds: &[],
            policy: &policy,
        });
        assert_ne!(p1.digest_urn, p2.digest_urn);
    }
}
