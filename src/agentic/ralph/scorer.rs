use crate::agentic::ralph::indexer::PortfolioState;
use anyhow::Result;

/// Maturity levels returned by [`MaturityScorer::evaluate`].
///
/// | Level | Name     | Meaning                                              |
/// |-------|----------|------------------------------------------------------|
/// | 0     | Nascent  | No artifacts or projects discovered                  |
/// | 1     | Emerging | Minimal footprint — bootstrap stage                  |
/// | 2     | Defined  | Some artifacts present, growing project count        |
/// | 3     | Managed  | Good coverage of artifacts across multiple projects  |
/// | 4     | Optimising | Strong portfolio with rich artifact diversity      |
/// | 5     | Autonomous | Full portfolio with receipts and ontology coverage |
pub struct MaturityScorer;

impl Default for MaturityScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl MaturityScorer {
    pub fn new() -> Self {
        Self
    }

    /// Score portfolio maturity on a 0-5 scale by weighting multiple
    /// dimensions derived from the scanned [`PortfolioState`]:
    ///
    /// - **Project density** (0-2 pts): how many active projects are present
    /// - **Artifact volume** (0-2 pts): total number of known artifacts
    /// - **Artifact diversity** (0-1 pt): mix of plans, ontology, receipts
    ///
    /// The resulting raw score is clamped to `[0, 5]`.
    pub fn evaluate(&self, state: &PortfolioState) -> Result<u8> {
        let mut score: u8 = 0;

        // ── Dimension 1: Project density ─────────────────────────────────────
        score += match state.active_projects {
            0 => 0,
            1..=2 => 1,
            _ => 2,
        };

        // ── Dimension 2: Artifact volume ─────────────────────────────────────
        let n = state.known_artifacts.len();
        score += match n {
            0 => 0,
            1..=4 => 1,
            _ => 2,
        };

        // ── Dimension 3: Artifact diversity ──────────────────────────────────
        // Award 1 point if the portfolio contains at least one plan file AND
        // at least one ontology file or receipt.
        let has_plan = state.known_artifacts.iter().any(|p| {
            p.parent()
                .and_then(|d| d.file_name())
                .map(|d| d == "plans")
                .unwrap_or(false)
                || p.extension().map(|e| e == "md").unwrap_or(false)
        });
        let has_onto_or_receipt = state.known_artifacts.iter().any(|p| {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            matches!(ext, "ttl" | "nt" | "rq")
                || p.parent()
                    .and_then(|d| d.file_name())
                    .map(|d| d == ".receipts")
                    .unwrap_or(false)
                || p.to_string_lossy().contains(".receipts")
        });
        if has_plan && has_onto_or_receipt {
            score += 1;
        }

        Ok(score.min(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_state(active_projects: usize, artifacts: &[&str]) -> PortfolioState {
        PortfolioState {
            active_projects,
            known_artifacts: artifacts.iter().map(|s| PathBuf::from(s)).collect(),
        }
    }

    #[test]
    fn test_evaluate_empty_state_is_zero() {
        let scorer = MaturityScorer::new();
        let state = make_state(0, &[]);
        assert_eq!(scorer.evaluate(&state).unwrap(), 0);
    }

    #[test]
    fn test_evaluate_single_project_no_artifacts_is_one() {
        let scorer = MaturityScorer::new();
        let state = make_state(1, &[]);
        assert_eq!(scorer.evaluate(&state).unwrap(), 1);
    }

    #[test]
    fn test_evaluate_many_projects_many_artifacts() {
        let scorer = MaturityScorer::new();
        let artifacts: Vec<&str> = vec![
            "/root/plans/001.md",
            "/root/plans/002.md",
            "/root/plans/003.md",
            "/root/plans/004.md",
            "/root/plans/005.md",
            "/root/ontologies/core.nt",
        ];
        let state = make_state(5, &artifacts);
        let level = scorer.evaluate(&state).unwrap();
        // 2 (projects) + 2 (artifacts >= 5) + 1 (has plan + ontology) = 5
        assert_eq!(level, 5);
    }

    #[test]
    fn test_evaluate_plans_only_no_diversity_bonus() {
        let scorer = MaturityScorer::new();
        let state = make_state(
            3,
            &[
                "/root/plans/001.md",
                "/root/plans/002.md",
                "/root/plans/003.md",
                "/root/plans/004.md",
                "/root/plans/005.md",
            ],
        );
        // 2 (projects > 2) + 2 (artifacts >= 5) + 0 (no onto/receipt) = 4
        assert_eq!(scorer.evaluate(&state).unwrap(), 4);
    }

    #[test]
    fn test_evaluate_result_never_exceeds_5() {
        let scorer = MaturityScorer::new();
        // Construct an extreme state
        let artifacts: Vec<&str> = (0..50)
            .map(|i| {
                if i % 2 == 0 {
                    "/root/plans/x.md"
                } else {
                    "/root/ontologies/y.nt"
                }
            })
            .collect();
        let state = make_state(100, &artifacts);
        assert!(scorer.evaluate(&state).unwrap() <= 5);
    }

    #[test]
    fn test_evaluate_two_projects_some_artifacts_no_diversity() {
        let scorer = MaturityScorer::new();
        let state = make_state(2, &["/root/Cargo.toml", "/root/dteam.toml"]);
        // 1 (1-2 projects) + 1 (1-4 artifacts) + 0 = 2
        assert_eq!(scorer.evaluate(&state).unwrap(), 2);
    }
}
