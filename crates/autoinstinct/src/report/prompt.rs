//! Build the strict-JSON prompt for Gemini.
//!
//! The prompt is hint-only; admission is the load-bearing surface.
//! Still, a precise prompt reduces wasted Gemini calls by making it
//! likely the model returns an admissible draft on the first try.

use crate::report::admit::FORBIDDEN_OVERCLAIMS;
use crate::report::evidence::{EvidenceBundle, REQUIRED_FILES};
use crate::report::schema::ReportKind;

/// Build the prompt body for `gemini -p <prompt>`.
#[must_use]
pub fn build_prompt(kind: ReportKind, bundle: &EvidenceBundle) -> String {
    let mut p = String::new();
    p.push_str(
        "You are generating an AutoInstinct anti-fake evidence report.\n\
You must only make claims supported by the supplied evidence bundle.\n\
Return ONLY valid JSON. Do not include markdown fences. Do not include prose outside JSON.\n\n",
    );
    p.push_str(&format!("Report kind: {}\n", kind.as_str()));
    p.push_str(&format!("Commit: {}\n", bundle.scorecard.commit_recorded));
    p.push_str(&format!("Toolchain: {}\n", bundle.scorecard.toolchain_recorded));
    p.push_str(&format!(
        "Overall status (from scorecard): {}\n\n",
        if bundle.scorecard.overall_pass { "PASS" } else { "FAIL" }
    ));

    p.push_str("The evidence bundle (concatenated on stdin) contains:\n");
    for f in REQUIRED_FILES {
        p.push_str(&format!("  - {f}\n"));
    }
    p.push('\n');

    p.push_str(
        "Return JSON matching exactly:\n\
{\n\
  \"reportKind\": \"<kind>\",\n\
  \"title\": \"<string>\",\n\
  \"commit\": \"<scorecard commit>\",\n\
  \"toolchain\": \"<scorecard toolchain>\",\n\
  \"overallStatus\": \"PASS|FAIL\",\n\
  \"claims\": [\n\
    {\n\
      \"id\": \"<stable id>\",\n\
      \"claim\": \"<claim text>\",\n\
      \"evidenceFiles\": [\"<file from bundle>\"],\n\
      \"evidenceSnippets\": [\"<exact substring present in cited file>\"],\n\
      \"riskIfFalse\": \"<what breaks if this claim is wrong>\"\n\
    }\n\
  ],\n\
  \"openRisks\": [\n\
    { \"id\": \"<id>\", \"risk\": \"<text>\", \"severity\": \"P0|P1|P2\", \"mitigation\": \"<text>\" }\n\
  ],\n\
  \"markdown\": \"<full markdown body>\"\n\
}\n\n",
    );

    p.push_str("Hard rules — admission rejects the report otherwise:\n");
    p.push_str("1. overallStatus must equal the scorecard's overall_pass.\n");
    p.push_str("2. commit must equal the scorecard's commit_recorded.\n");
    p.push_str("3. Every claim must cite at least one evidence file AND at least one snippet that is a substring of that file.\n");
    p.push_str("4. Master/end-to-end claims must cite anti_fake_master.out.\n");
    p.push_str("5. Pack runtime / matched_rule_id claims must cite anti_fake_packs.out.\n");
    p.push_str("6. Zero-allocation claims must cite anti_fake_perf.out.\n");
    p.push_str("7. OCEL admission claims must cite anti_fake_ocel.out.\n");
    p.push_str("8. Do NOT use any of these forbidden over-claim phrases (case-insensitive):\n");
    for term in FORBIDDEN_OVERCLAIMS {
        p.push_str("   - ");
        p.push_str(term);
        p.push('\n');
    }
    p.push_str(
        "9. Prefer language: \"anti-fake substrate complete\", \"master loop proven\",\n   \
\"runtime pack contribution proven\", \"enterprise operational hardening remains\".\n\
10. Distinguish substrate-complete from enterprise-operational-complete.\n\
11. Do not invent test names; use only test names visible in the evidence outputs.\n",
    );
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scorecard::all_true_scorecard;

    fn dummy_bundle() -> EvidenceBundle {
        let mut outputs = std::collections::BTreeMap::new();
        for f in REQUIRED_FILES {
            outputs.insert((*f).to_string(), format!("body of {f}\n"));
        }
        EvidenceBundle {
            root: std::path::PathBuf::from("/tmp/x"),
            scorecard: all_true_scorecard(),
            git_txt: String::new(),
            toolchain_txt: String::new(),
            outputs,
        }
    }

    #[test]
    fn prompt_lists_every_required_file() {
        let p = build_prompt(ReportKind::Executive, &dummy_bundle());
        for f in REQUIRED_FILES {
            assert!(p.contains(f), "prompt must list {f}");
        }
    }

    #[test]
    fn prompt_lists_every_overclaim_term() {
        let p = build_prompt(ReportKind::Audit, &dummy_bundle());
        for term in FORBIDDEN_OVERCLAIMS {
            assert!(p.contains(term), "prompt must list forbidden term: {term}");
        }
    }
}
