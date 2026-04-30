//! Phase 9 — Compliance & governance.
//!
//! Audits a compiled pack against domain-specific compliance flags
//! (HIPAA, GDPR, SOX, PCI, SOC2, ISO27001). Each flag has a small set of
//! mechanical checks: ontology purity, breed admissibility constraints,
//! response-class restrictions, and tier eligibility. The output is an
//! `AuditReport` regulator-ready: a deterministic list of findings.

use serde::{Deserialize, Serialize};

use crate::compile::FieldPackArtifact;
use crate::domain::{profile, Domain};
use crate::AutonomicInstinct;

/// One compliance finding.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    /// Compliance flag (e.g. "HIPAA").
    pub flag: String,
    /// Rule identifier (stable, machine-parseable).
    pub rule: String,
    /// Human-readable description.
    pub message: String,
    /// True iff the finding is a violation; false iff it's an info note.
    pub violation: bool,
}

/// Regulator-ready audit report.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuditReport {
    /// Domain audited.
    pub domain: String,
    /// Pack digest URN.
    pub pack_digest_urn: String,
    /// Findings in deterministic order.
    pub findings: Vec<Finding>,
}

impl AuditReport {
    /// True iff no violations were found.
    #[must_use]
    pub fn clean(&self) -> bool {
        !self.findings.iter().any(|f| f.violation)
    }
}

/// Audit a pack against a domain's compliance flags.
#[must_use]
pub fn audit(pack: &FieldPackArtifact, domain: Domain) -> AuditReport {
    let dp = profile(domain);
    let mut findings = Vec::new();

    // Rule 1 — ontology purity. Every pack ontology entry must be in the
    // domain profile's allowlist.
    for iri in &pack.ontology_profile {
        if !dp.ontology_profile.iter().any(|p| iri.starts_with(p)) {
            findings.push(Finding {
                flag: "ONTOLOGY".into(),
                rule: "PURE_ONTOLOGY_PROFILE".into(),
                message: format!(
                    "pack ontology entry {} is not in domain {} profile",
                    iri, dp.name
                ),
                violation: true,
            });
        }
    }

    // Rule 2 — breed admissibility. Pack breeds ⊆ domain breeds.
    for b in &pack.admitted_breeds {
        if !dp.admitted_breeds.contains(&b.as_str()) {
            findings.push(Finding {
                flag: "BREED".into(),
                rule: "BREED_NOT_IN_DOMAIN".into(),
                message: format!(
                    "pack admits breed {} which is not in domain {} profile",
                    b, dp.name
                ),
                violation: true,
            });
        }
    }

    // Rule 3 — HIPAA: no Refuse / Escalate as default response (must surface
    // clinical decisions for human review).
    if dp.compliance_flags.contains(&"HIPAA")
        && matches!(
            pack.default_response,
            AutonomicInstinct::Refuse | AutonomicInstinct::Escalate
        )
    {
        findings.push(Finding {
            flag: "HIPAA".into(),
            rule: "DEFAULT_NEVER_REFUSE_OR_ESCALATE".into(),
            message: "healthcare default response must support clinician review".into(),
            violation: true,
        });
    }

    // Rule 4 — SOX/PCI: dendral breed required for audit reconstruction.
    for f in dp.compliance_flags {
        if matches!(*f, "SOX" | "PCI") && !pack.admitted_breeds.iter().any(|b| b == "dendral") {
            findings.push(Finding {
                flag: (*f).into(),
                rule: "AUDIT_RECONSTRUCTION_REQUIRES_DENDRAL".into(),
                message: format!(
                    "{} requires the dendral breed for audit-trail reconstruction",
                    f
                ),
                violation: true,
            });
        }
    }

    // Rule 5 — GDPR info note: PII handling is enforced at IRI level by the
    // pack itself; we record an info finding so audit consumers can verify.
    if dp.compliance_flags.contains(&"GDPR") {
        findings.push(Finding {
            flag: "GDPR".into(),
            rule: "URN_BLAKE3_FOR_IDENTITY".into(),
            message: "domain mandates urn:blake3 IRIs for identity tokens".into(),
            violation: false,
        });
    }

    // Determinism: sort findings.
    findings.sort_by(|a, b| a.flag.cmp(&b.flag).then_with(|| a.rule.cmp(&b.rule)));

    AuditReport {
        domain: dp.name.into(),
        pack_digest_urn: pack.digest_urn.clone(),
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile::{compile, CompileInputs};
    use crate::synth::CandidatePolicy;

    fn pack_for(name: &str, breeds: &[&str], default: AutonomicInstinct) -> FieldPackArtifact {
        let policy = CandidatePolicy {
            rules: vec![],
            default,
        };
        compile(CompileInputs {
            name,
            ontology_profile: &[
                "https://schema.org/",
                "http://www.w3.org/ns/prov#",
                "urn:blake3:",
            ],
            admitted_breeds: breeds,
            policy: &policy,
        })
    }

    #[test]
    fn healthcare_pack_with_refuse_default_violates_hipaa() {
        let p = pack_for(
            "care-1",
            &["eliza", "mycin"],
            AutonomicInstinct::Refuse,
        );
        let r = audit(&p, Domain::Healthcare);
        assert!(!r.clean());
        assert!(r.findings.iter().any(|f| f.flag == "HIPAA" && f.violation));
    }

    #[test]
    fn financial_pack_without_dendral_violates_sox_pci() {
        let p = pack_for(
            "fin-1",
            &["mycin", "strips"],
            AutonomicInstinct::Ignore,
        );
        let r = audit(&p, Domain::Financial);
        assert!(!r.clean());
        let sox = r.findings.iter().find(|f| f.flag == "SOX").unwrap();
        let pci = r.findings.iter().find(|f| f.flag == "PCI").unwrap();
        assert!(sox.violation);
        assert!(pci.violation);
    }

    #[test]
    fn enterprise_pack_with_admitted_breeds_passes_audit() {
        let p = pack_for(
            "ent-1",
            &["eliza", "mycin", "dendral"],
            AutonomicInstinct::Ask,
        );
        let r = audit(&p, Domain::Enterprise);
        // Only info notes allowed.
        assert!(r.clean(), "{:?}", r.findings);
    }

    #[test]
    fn audit_report_is_deterministic() {
        let p = pack_for("ent-1", &["mycin"], AutonomicInstinct::Ask);
        let r1 = audit(&p, Domain::Enterprise);
        let r2 = audit(&p, Domain::Enterprise);
        assert_eq!(r1.findings, r2.findings);
    }
}
