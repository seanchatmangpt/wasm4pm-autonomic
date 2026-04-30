//! KZ8 admission gauntlet for Gemini-authored reports.
//!
//! Admission is the load-bearing surface — the prompt is hint-only.
//! Every claim must cite a real evidence file and a snippet that is
//! actually present in that file. Status and commit must align with
//! the supplied scorecard. Forbidden over-claims are rejected wherever
//! they appear (markdown, claim text, risk text).

use thiserror::Error;

use crate::report::evidence::EvidenceBundle;
use crate::report::schema::{GeneratedReport, ReportStatus};

/// Forbidden over-claim phrases. A single occurrence in any text field
/// (markdown, claim, claim text, risk text, mitigation) → rejection.
pub const FORBIDDEN_OVERCLAIMS: &[&str] = &[
    "SOC2 certified",
    "SOC 2 certified",
    "HIPAA compliant",
    "FedRAMP ready",
    "Fortune 5 SLA ready",
    "production certified",
    "regulator approved",
    "fully complete",
    "unfakeable",
    "unhackable",
];

/// Reasons a report may be refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReportAdmissionError {
    /// Report's overall_status does not match the scorecard.
    #[error("status mismatch: report={report:?} scorecard_overall_pass={scorecard}")]
    StatusMismatch {
        /// Status the report carried.
        report: ReportStatus,
        /// `Scorecard::overall_pass` from the bundle.
        scorecard: bool,
    },
    /// Report commit ≠ scorecard commit.
    #[error("commit mismatch: report={report} scorecard={scorecard}")]
    CommitMismatch {
        /// Commit the report carried.
        report: String,
        /// Commit the bundle carried.
        scorecard: String,
    },
    /// Report has zero claims.
    #[error("report has zero claims")]
    NoClaims,
    /// A claim cited an evidence file that does not exist in the bundle.
    #[error("claim `{claim_id}` cites unknown evidence file: {file}")]
    EvidenceFileMissing {
        /// Offending claim id.
        claim_id: String,
        /// Cited file name.
        file: String,
    },
    /// A claim's snippet was not found in the cited evidence file.
    #[error("claim `{claim_id}` snippet not found in {file}: {snippet}")]
    SnippetNotFound {
        /// Offending claim id.
        claim_id: String,
        /// File where the snippet was expected.
        file: String,
        /// Snippet substring.
        snippet: String,
    },
    /// A claim cited zero files OR zero snippets.
    #[error("claim `{claim_id}` lacks citation: {what}")]
    EmptyCitation {
        /// Offending claim id.
        claim_id: String,
        /// Which dimension was empty (`evidence_files` or `evidence_snippets`).
        what: &'static str,
    },
    /// A forbidden over-claim term appeared in a text field.
    #[error("forbidden over-claim: `{0}`")]
    Overclaim(String),
    /// A claim required to cite a specific output file did not.
    #[error("claim `{claim_id}` must cite {required}: {reason}")]
    MissingRequiredCitation {
        /// Offending claim id.
        claim_id: String,
        /// Required file the claim should have cited.
        required: &'static str,
        /// Why the rule fired.
        reason: &'static str,
    },
    /// Markdown body is empty.
    #[error("markdown body is empty")]
    EmptyMarkdown,
}

/// Citation rule: if any keyword in `triggers` appears (case-insensitive)
/// in a claim's text, the claim must cite `required_file`.
struct CitationRule {
    required_file: &'static str,
    triggers: &'static [&'static str],
    reason: &'static str,
}

const CITATION_RULES: &[CitationRule] = &[
    CitationRule {
        required_file: "anti_fake_master.out",
        triggers: &["master loop", "end-to-end loop", "ocel-to-pack"],
        reason: "claims about the master/end-to-end loop must cite anti_fake_master.out",
    },
    CitationRule {
        required_file: "anti_fake_packs.out",
        triggers: &[
            "runtime pack",
            "matched_rule_id",
            "matched_pack_id",
            "select_instinct_with_pack",
            "pack runtime",
        ],
        reason: "claims about pack runtime / matched-rule metadata must cite anti_fake_packs.out",
    },
    CitationRule {
        required_file: "anti_fake_perf.out",
        triggers: &["zero allocation", "zero-alloc", "alloc-free"],
        reason: "claims about zero allocation must cite anti_fake_perf.out",
    },
    CitationRule {
        required_file: "anti_fake_ocel.out",
        triggers: &["ocel admission", "ocel authenticity", "world admission"],
        reason: "claims about OCEL admission must cite anti_fake_ocel.out",
    },
];

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
}

/// Run KZ8 admission against a report and its evidence bundle.
///
/// # Errors
///
/// Returns the first [`ReportAdmissionError`] encountered.
pub fn admit(report: &GeneratedReport, bundle: &EvidenceBundle) -> Result<(), ReportAdmissionError> {
    if report.markdown.trim().is_empty() {
        return Err(ReportAdmissionError::EmptyMarkdown);
    }
    if report.claims.is_empty() {
        return Err(ReportAdmissionError::NoClaims);
    }

    // Status alignment.
    let scorecard_pass = bundle.scorecard.overall_pass;
    let report_pass = matches!(report.overall_status, ReportStatus::Pass);
    if report_pass != scorecard_pass {
        return Err(ReportAdmissionError::StatusMismatch {
            report: report.overall_status,
            scorecard: scorecard_pass,
        });
    }

    // Commit alignment.
    if report.commit != bundle.scorecard.commit_recorded {
        return Err(ReportAdmissionError::CommitMismatch {
            report: report.commit.clone(),
            scorecard: bundle.scorecard.commit_recorded.clone(),
        });
    }

    // Per-claim citation + snippet verification.
    for claim in &report.claims {
        if claim.evidence_files.is_empty() {
            return Err(ReportAdmissionError::EmptyCitation {
                claim_id: claim.id.clone(),
                what: "evidence_files",
            });
        }
        if claim.evidence_snippets.is_empty() {
            return Err(ReportAdmissionError::EmptyCitation {
                claim_id: claim.id.clone(),
                what: "evidence_snippets",
            });
        }
        for file in &claim.evidence_files {
            if bundle.get(file).is_none() {
                return Err(ReportAdmissionError::EvidenceFileMissing {
                    claim_id: claim.id.clone(),
                    file: file.clone(),
                });
            }
        }
        // Each snippet must appear in at least one of the cited files.
        for snippet in &claim.evidence_snippets {
            let found = claim
                .evidence_files
                .iter()
                .any(|f| bundle.get(f).is_some_and(|body| body.contains(snippet)));
            if !found {
                return Err(ReportAdmissionError::SnippetNotFound {
                    claim_id: claim.id.clone(),
                    // Attribute to first cited file for the error message.
                    file: claim.evidence_files[0].clone(),
                    snippet: snippet.clone(),
                });
            }
        }

        // Kind-specific citation rules.
        for rule in CITATION_RULES {
            let triggered = rule
                .triggers
                .iter()
                .any(|t| contains_ci(&claim.claim, t) || contains_ci(&claim.id, t));
            if triggered
                && !claim
                    .evidence_files
                    .iter()
                    .any(|f| f == rule.required_file)
            {
                return Err(ReportAdmissionError::MissingRequiredCitation {
                    claim_id: claim.id.clone(),
                    required: rule.required_file,
                    reason: rule.reason,
                });
            }
        }
    }

    // Forbidden over-claims — scan markdown + every text field.
    for term in FORBIDDEN_OVERCLAIMS {
        if contains_ci(&report.markdown, term) {
            return Err(ReportAdmissionError::Overclaim((*term).to_string()));
        }
        for c in &report.claims {
            if contains_ci(&c.claim, term) || contains_ci(&c.risk_if_false, term) {
                return Err(ReportAdmissionError::Overclaim((*term).to_string()));
            }
        }
        for r in &report.open_risks {
            if contains_ci(&r.risk, term) || contains_ci(&r.mitigation, term) {
                return Err(ReportAdmissionError::Overclaim((*term).to_string()));
            }
        }
    }

    Ok(())
}
