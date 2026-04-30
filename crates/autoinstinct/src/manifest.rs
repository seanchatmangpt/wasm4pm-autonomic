//! Phase 9 — Field-pack manifest exporter.
//!
//! A `PackManifest` is the portable, signable, replayable summary of a
//! compiled field pack. It carries the pack digest, ontology profile,
//! admitted breeds, response-class summary, and the BLAKE3 of the
//! canonical bytes (for tamper detection by external auditors).

use serde::{Deserialize, Serialize};

use crate::compile::FieldPackArtifact;
use crate::AutonomicInstinct;

/// Portable manifest for an external auditor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackManifest {
    /// Pack name.
    pub name: String,
    /// AutoInstinct compiler version.
    pub autoinstinct_version: String,
    /// Pack digest URN (`urn:blake3:`).
    pub digest_urn: String,
    /// Ontology profile.
    pub ontology_profile: Vec<String>,
    /// Admitted breeds.
    pub admitted_breeds: Vec<String>,
    /// Counts of rules per response class. Deterministic key order.
    pub rules_by_class: Vec<(AutonomicInstinct, usize)>,
    /// Default response.
    pub default_response: AutonomicInstinct,
    /// BLAKE3 of the canonical manifest bytes (excluding this field).
    pub manifest_digest_urn: String,
}

/// Build a manifest from a compiled artifact.
#[must_use]
pub fn build(pack: &FieldPackArtifact) -> PackManifest {
    let mut counts: indexmap::IndexMap<AutonomicInstinct, usize> = indexmap::IndexMap::new();
    for (_, r) in &pack.rules {
        *counts.entry(*r).or_insert(0) += 1;
    }
    let mut rules_by_class: Vec<(AutonomicInstinct, usize)> = counts.into_iter().collect();
    rules_by_class.sort_by_key(|(r, _)| *r as u8);

    let mut manifest = PackManifest {
        name: pack.name.clone(),
        autoinstinct_version: pack.autoinstinct_version.clone(),
        digest_urn: pack.digest_urn.clone(),
        ontology_profile: pack.ontology_profile.clone(),
        admitted_breeds: pack.admitted_breeds.clone(),
        rules_by_class,
        default_response: pack.default_response,
        manifest_digest_urn: String::new(),
    };
    let canonical = canonical_bytes(&manifest);
    let h = blake3::hash(&canonical);
    manifest.manifest_digest_urn = format!("urn:blake3:{}", h.to_hex());
    manifest
}

/// Canonical bytes used for the manifest digest.
fn canonical_bytes(m: &PackManifest) -> Vec<u8> {
    let mut clone = m.clone();
    clone.manifest_digest_urn.clear();
    serde_json::to_vec(&clone).expect("PackManifest is always serializable")
}

/// Verify a manifest's `manifest_digest_urn` matches its content.
#[must_use]
pub fn verify(m: &PackManifest) -> bool {
    let mut clone = m.clone();
    clone.manifest_digest_urn.clear();
    let bytes = serde_json::to_vec(&clone).expect("serializable");
    let h = blake3::hash(&bytes);
    m.manifest_digest_urn == format!("urn:blake3:{}", h.to_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile, CompileInputs};
    use crate::synth::CandidatePolicy;

    fn sample_pack() -> FieldPackArtifact {
        let policy = CandidatePolicy {
            rules: vec![
                ("urn:blake3:a".into(), AutonomicInstinct::Ask),
                ("urn:blake3:b".into(), AutonomicInstinct::Inspect),
                ("urn:blake3:c".into(), AutonomicInstinct::Ask),
            ],
            default: AutonomicInstinct::Ignore,
        };
        compile(CompileInputs {
            name: "manifest-test",
            ontology_profile: &["urn:blake3:"],
            admitted_breeds: &["mycin"],
            policy: &policy,
        })
    }

    #[test]
    fn manifest_roundtrips_and_verifies() {
        let pack = sample_pack();
        let manifest = build(&pack);
        assert!(verify(&manifest));
        assert!(manifest.manifest_digest_urn.starts_with("urn:blake3:"));
    }

    #[test]
    fn manifest_counts_rules_by_class() {
        let pack = sample_pack();
        let manifest = build(&pack);
        let ask_count = manifest
            .rules_by_class
            .iter()
            .find(|(r, _)| *r == AutonomicInstinct::Ask)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        assert_eq!(ask_count, 2, "two rules emit Ask");
    }

    #[test]
    fn manifest_tamper_matrix() {
        let pack = sample_pack();
        let base_manifest = build(&pack);

        // 1. Name tamper
        let mut m = base_manifest.clone();
        m.name.push('!');
        assert!(!verify(&m), "name tamper must invalidate manifest");

        // 2. Version tamper
        let mut m = base_manifest.clone();
        m.autoinstinct_version = "99.99.99".into();
        assert!(!verify(&m), "version tamper must invalidate manifest");

        // 3. Digest tamper
        let mut m = base_manifest.clone();
        m.digest_urn = "urn:blake3:deadbeef".into();
        assert!(!verify(&m), "digest tamper must invalidate manifest");

        // 4. Ontology Profile tamper
        let mut m = base_manifest.clone();
        m.ontology_profile.push("urn:acme:fake".into());
        assert!(!verify(&m), "ontology profile tamper must invalidate manifest");

        // 5. Admitted Breeds tamper
        let mut m = base_manifest.clone();
        m.admitted_breeds.push("fake_breed".into());
        assert!(!verify(&m), "admitted breeds tamper must invalidate manifest");

        // 6. Rules by class tamper
        let mut m = base_manifest.clone();
        if let Some(first) = m.rules_by_class.first_mut() {
            first.1 += 1;
        }
        assert!(!verify(&m), "rules_by_class tamper must invalidate manifest");

        // 7. Default response tamper
        let mut m = base_manifest.clone();
        m.default_response = AutonomicInstinct::Escalate;
        assert!(!verify(&m), "default_response tamper must invalidate manifest");
    }
}
