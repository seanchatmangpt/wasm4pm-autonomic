use crate::agentic::ralph::indexer::PortfolioState;
use anyhow::Result;

pub struct WorkSelector;

impl Default for WorkSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkSelector {
    pub fn new() -> Self {
        Self
    }

    /// Select the next admissible unit of work from `state`.
    ///
    /// Priority order (highest first):
    ///
    /// 1. **No projects found** → bootstrap: initialise the portfolio manifest.
    /// 2. **No plan artifacts** → discovery: specify the first idea from the
    ///    plans directory or create an initial plan.
    /// 3. **No ontology or receipt artifacts** → ontology closure: generate
    ///    the public ontology namespaces so downstream operators can ground
    ///    their receipts.
    /// 4. **Few artifacts (< 5)** → expand: add more plan files or ontology
    ///    entries to grow the known artifact surface.
    /// 5. **Mature portfolio** → optimise: run the meta-analysis loop to
    ///    improve the existing topology.
    ///
    /// The returned string is a human-readable imperative sentence that can be
    /// fed directly into a Ralph prompt.
    pub fn select_next(&self, state: &PortfolioState) -> Result<String> {
        if state.active_projects == 0 {
            return Ok(
                "Bootstrap: create PROGRAM-CHARTER.md and dteam.toml to establish the portfolio manifest.".to_string(),
            );
        }

        let has_plan = state.known_artifacts.iter().any(|p| {
            p.parent()
                .and_then(|d| d.file_name())
                .map(|d| d == "plans")
                .unwrap_or(false)
                || p.extension().map(|e| e == "md").unwrap_or(false)
        });

        if !has_plan {
            return Ok(
                "Discovery: specify the first idea — create plans/001-initial-idea.md with a clear problem statement and acceptance criteria.".to_string(),
            );
        }

        let has_onto = state.known_artifacts.iter().any(|p| {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            matches!(ext, "ttl" | "nt" | "rq")
        });

        if !has_onto {
            return Ok(
                "Ontology closure: generate ontologies/core.nt with public namespace anchors so receipts can be semantically grounded.".to_string(),
            );
        }

        let has_receipt = state.known_artifacts.iter().any(|p| {
            p.to_string_lossy().contains(".receipts")
                || p.parent()
                    .and_then(|d| d.file_name())
                    .map(|d| d == ".receipts")
                    .unwrap_or(false)
        });

        if !has_receipt {
            return Ok(
                "Receipt emission: run the first full pipeline pass and emit a receipt so provenance is anchored.".to_string(),
            );
        }

        if state.known_artifacts.len() < 5 {
            return Ok(format!(
                "Expand: {} artifact(s) found across {} project(s) — add more plan specifications or ontology entries to reach minimum viable coverage.",
                state.known_artifacts.len(),
                state.active_projects,
            ));
        }

        // Mature portfolio — propose optimisation based on current size.
        let plan_count = state.known_artifacts.iter().filter(|p| {
            p.parent()
                .and_then(|d| d.file_name())
                .map(|d| d == "plans")
                .unwrap_or(false)
        }).count();

        Ok(format!(
            "Optimise: {} project(s), {} artifact(s), {} plan(s) — run meta-analysis to refine topology and close ontology gaps.",
            state.active_projects,
            state.known_artifacts.len(),
            plan_count,
        ))
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
    fn test_select_next_no_projects_bootstraps() {
        let sel = WorkSelector::new();
        let state = make_state(0, &[]);
        let unit = sel.select_next(&state).unwrap();
        assert!(
            unit.to_lowercase().contains("bootstrap"),
            "expected bootstrap directive, got: {unit}"
        );
    }

    #[test]
    fn test_select_next_no_plans_does_discovery() {
        let sel = WorkSelector::new();
        let state = make_state(1, &["/root/Cargo.toml"]);
        let unit = sel.select_next(&state).unwrap();
        assert!(
            unit.to_lowercase().contains("discovery"),
            "expected discovery directive, got: {unit}"
        );
    }

    #[test]
    fn test_select_next_no_ontology_does_closure() {
        let sel = WorkSelector::new();
        let state = make_state(1, &["/root/plans/001.md"]);
        let unit = sel.select_next(&state).unwrap();
        assert!(
            unit.to_lowercase().contains("ontology"),
            "expected ontology directive, got: {unit}"
        );
    }

    #[test]
    fn test_select_next_no_receipt_does_emission() {
        let sel = WorkSelector::new();
        let state = make_state(
            1,
            &["/root/plans/001.md", "/root/ontologies/core.nt"],
        );
        let unit = sel.select_next(&state).unwrap();
        assert!(
            unit.to_lowercase().contains("receipt"),
            "expected receipt directive, got: {unit}"
        );
    }

    #[test]
    fn test_select_next_few_artifacts_expands() {
        let sel = WorkSelector::new();
        let state = make_state(
            1,
            &[
                "/root/plans/001.md",
                "/root/ontologies/core.nt",
                "/root/.receipts/receipt_1.txt",
            ],
        );
        let unit = sel.select_next(&state).unwrap();
        assert!(
            unit.to_lowercase().contains("expand"),
            "expected expand directive, got: {unit}"
        );
    }

    #[test]
    fn test_select_next_mature_portfolio_optimises() {
        let sel = WorkSelector::new();
        let state = make_state(
            3,
            &[
                "/root/plans/001.md",
                "/root/plans/002.md",
                "/root/ontologies/core.nt",
                "/root/.receipts/receipt_1.txt",
                "/root/dteam.toml",
            ],
        );
        let unit = sel.select_next(&state).unwrap();
        assert!(
            unit.to_lowercase().contains("optimis"),
            "expected optimise directive, got: {unit}"
        );
    }

    #[test]
    fn test_select_next_result_is_not_empty() {
        let sel = WorkSelector::new();
        for n in 0..=5_usize {
            let artifacts: Vec<String> = (0..n).map(|i| format!("/root/artifact_{i}.md")).collect();
            let artifact_strs: Vec<&str> = artifacts.iter().map(|s| s.as_str()).collect();
            let state = make_state(n, &artifact_strs);
            let unit = sel.select_next(&state).unwrap();
            assert!(!unit.is_empty(), "select_next must never return empty string");
        }
    }
}
