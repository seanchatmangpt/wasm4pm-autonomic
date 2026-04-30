//! Strict serde schema for Gemini-authored anti-fake reports (KZ8).
//!
//! Every struct uses `deny_unknown_fields` so an over-eager LLM cannot
//! introduce silent fields. Field naming is camelCase to match the
//! prompt contract Gemini sees.

use serde::{Deserialize, Serialize};

/// Audience kind for a generated report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    /// Executive / board / buyer audience.
    Executive,
    /// Technical / engineering reviewer audience.
    Technical,
    /// Auditor / compliance / due diligence audience.
    Audit,
    /// Release notes audience.
    Release,
    /// Regression-diff comparison report.
    RegressionDiff,
}

impl ReportKind {
    /// Stable string id used in prompts and CLI flags.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ReportKind::Executive => "executive",
            ReportKind::Technical => "technical",
            ReportKind::Audit => "audit",
            ReportKind::Release => "release",
            ReportKind::RegressionDiff => "regression_diff",
        }
    }

    /// Parse from CLI string. Errors if unrecognized.
    ///
    /// # Errors
    ///
    /// Returns the offending input if it does not match a known kind.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "executive" => Ok(Self::Executive),
            "technical" => Ok(Self::Technical),
            "audit" => Ok(Self::Audit),
            "release" => Ok(Self::Release),
            "regression_diff" | "regression-diff" => Ok(Self::RegressionDiff),
            other => Err(other.to_string()),
        }
    }
}

/// Overall status carried by the report header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReportStatus {
    /// Every required scorecard dimension passed.
    Pass,
    /// At least one required scorecard dimension failed.
    Fail,
}

/// Severity for [`OpenRisk`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskSeverity {
    /// P0 — release-blocking.
    P0,
    /// P1 — significant but not release-blocking.
    P1,
    /// P2 — minor / nice-to-have.
    P2,
}

/// One supported claim in the report. Every claim must cite ≥1 evidence
/// file AND ≥1 substring snippet that occurs in that file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportClaim {
    /// Stable claim id (e.g. `master.loop.proven`).
    pub id: String,
    /// Plain-language claim text.
    pub claim: String,
    /// Evidence file names cited (relative to the evidence directory).
    pub evidence_files: Vec<String>,
    /// Substrings that the admission layer verifies are present in the
    /// cited files.
    pub evidence_snippets: Vec<String>,
    /// What goes wrong if the claim is false / unsupported.
    pub risk_if_false: String,
}

/// One open risk that the report must surface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenRisk {
    /// Stable risk id (e.g. `enterprise.production.hardening`).
    pub id: String,
    /// Plain-language risk text.
    pub risk: String,
    /// Severity classification.
    pub severity: RiskSeverity,
    /// Mitigation strategy or open follow-up.
    pub mitigation: String,
}

/// Full Gemini-authored anti-fake report envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedReport {
    /// Report audience kind.
    pub report_kind: ReportKind,
    /// Title of the report.
    pub title: String,
    /// Commit the report references.
    pub commit: String,
    /// Toolchain string.
    pub toolchain: String,
    /// Overall status — must agree with the scorecard.
    pub overall_status: ReportStatus,
    /// Supported claims (≥1 required).
    pub claims: Vec<ReportClaim>,
    /// Open risks (may be empty).
    pub open_risks: Vec<OpenRisk>,
    /// Final markdown body. The render layer writes this verbatim only
    /// after admission has succeeded.
    pub markdown: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_kind_round_trip() {
        for (k, s) in [
            (ReportKind::Executive, "executive"),
            (ReportKind::Technical, "technical"),
            (ReportKind::Audit, "audit"),
            (ReportKind::Release, "release"),
            (ReportKind::RegressionDiff, "regression_diff"),
        ] {
            assert_eq!(k.as_str(), s);
            assert_eq!(ReportKind::parse(s).unwrap(), k);
        }
    }

    #[test]
    fn report_kind_parse_rejects_unknown() {
        assert!(ReportKind::parse("propaganda").is_err());
    }

    #[test]
    fn claim_unknown_field_is_rejected() {
        let bad = r#"{
            "id":"x","claim":"y","evidenceFiles":[],
            "evidenceSnippets":[],"riskIfFalse":"z","extra":1
        }"#;
        assert!(serde_json::from_str::<ReportClaim>(bad).is_err());
    }

    #[test]
    fn report_unknown_field_is_rejected() {
        let bad = r#"{
            "reportKind":"executive","title":"t","commit":"c",
            "toolchain":"r","overallStatus":"PASS",
            "claims":[],"openRisks":[],"markdown":"m","extra":1
        }"#;
        assert!(serde_json::from_str::<GeneratedReport>(bad).is_err());
    }

    #[test]
    fn report_round_trips() {
        let r = GeneratedReport {
            report_kind: ReportKind::Executive,
            title: "t".into(),
            commit: "c".into(),
            toolchain: "tc".into(),
            overall_status: ReportStatus::Pass,
            claims: vec![ReportClaim {
                id: "c1".into(),
                claim: "x".into(),
                evidence_files: vec!["scorecard.json".into()],
                evidence_snippets: vec!["overall_pass".into()],
                risk_if_false: "drift".into(),
            }],
            open_risks: vec![OpenRisk {
                id: "r1".into(),
                risk: "ops".into(),
                severity: RiskSeverity::P1,
                mitigation: "follow up".into(),
            }],
            markdown: "# t\n".into(),
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: GeneratedReport = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
