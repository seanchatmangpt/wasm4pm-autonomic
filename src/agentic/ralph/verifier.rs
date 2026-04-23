use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait DoDVerifier {
    fn verify(&self, working_dir: &Path) -> Result<()>;
}

#[derive(Default)]
pub struct CargoVerifier;

impl DoDVerifier for CargoVerifier {
    fn verify(&self, working_dir: &Path) -> Result<()> {
        let check_status = Command::new("cargo")
            .arg("check")
            .current_dir(working_dir)
            .output()?;

        let test_status = Command::new("cargo")
            .args(["test", "--lib"])
            .current_dir(working_dir)
            .output()?;

        if check_status.status.success() && test_status.status.success() {
            Ok(())
        } else {
            let err_out = if !check_status.status.success() {
                String::from_utf8_lossy(&check_status.stderr).into_owned()
            } else {
                String::from_utf8_lossy(&test_status.stdout).into_owned()
            };
            Err(anyhow!("Verification failed: {}", err_out))
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// AutoML / HDIT pipeline verifier
// ────────────────────────────────────────────────────────────────────────────

/// Outcome of verifying a single AutoML plan JSON file.
#[derive(Debug, Clone)]
pub struct PlanCheck {
    pub path: PathBuf,
    pub passed: bool,
    pub failures: Vec<String>,
}

/// Summary of verifying all AutoML pipeline artifacts.
#[derive(Debug, Clone, Default)]
pub struct AutomlDodReport {
    pub plans_checked: usize,
    pub plans_passed: usize,
    pub plans_failed: usize,
    pub failed_plans: Vec<PlanCheck>,
    pub summary_json_present: bool,
    pub summary_json_balanced: bool,
    pub config_has_ensemble_only: bool,
}

impl AutomlDodReport {
    pub fn is_pass(&self) -> bool {
        self.plans_failed == 0
            && self.summary_json_present
            && self.summary_json_balanced
            && !self.config_has_ensemble_only
    }
}

/// Verifies AutoML pipeline artifacts: every plan JSON has balanced accounting,
/// exactly one chosen Pareto candidate, all required fields. The summary file
/// exists and balances globally. The config doesn't contain the banned
/// `ensemble_only` strategy.
///
/// Anti-lie doctrine: this verifier PANICS or returns Err on any artifact that
/// fails an invariant — silent success on corrupt state is forbidden.
pub struct AutomlPipelineVerifier {
    pub plans_dir: PathBuf,
    pub summary_path: PathBuf,
    pub config_path: PathBuf,
}

impl AutomlPipelineVerifier {
    pub fn new(working_dir: &Path) -> Self {
        Self {
            plans_dir: working_dir.join("artifacts/pdc2025/automl_plans"),
            summary_path: working_dir.join("artifacts/pdc2025/automl_summary.json"),
            config_path: working_dir.join("dteam.toml"),
        }
    }

    pub fn report(&self) -> Result<AutomlDodReport> {
        let mut report = AutomlDodReport::default();

        // 1. Config: ensemble_only must NOT be present (banned after TPOT2 work)
        if self.config_path.exists() {
            let cfg = std::fs::read_to_string(&self.config_path)
                .with_context(|| format!("reading {:?}", self.config_path))?;
            for line in cfg.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("strategy") && trimmed.contains("ensemble_only") {
                    report.config_has_ensemble_only = true;
                }
            }
        }

        // 2. Every plan JSON validated
        if self.plans_dir.exists() {
            let mut plan_files: Vec<PathBuf> = std::fs::read_dir(&self.plans_dir)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
                .collect();
            plan_files.sort();

            for path in plan_files {
                let check = verify_plan_file(&path);
                report.plans_checked += 1;
                if check.passed {
                    report.plans_passed += 1;
                } else {
                    report.plans_failed += 1;
                    report.failed_plans.push(check);
                }
            }
        }

        // 3. Summary JSON: exists and accounting_balanced_global=true
        if self.summary_path.exists() {
            report.summary_json_present = true;
            let content = std::fs::read_to_string(&self.summary_path)
                .with_context(|| format!("reading {:?}", self.summary_path))?;
            let summary: serde_json::Value = serde_json::from_str(&content)
                .with_context(|| format!("parsing summary {:?}", self.summary_path))?;
            report.summary_json_balanced = summary
                .get("accounting_balanced_global")
                .and_then(|v| v.as_bool())
                == Some(true);
        }

        Ok(report)
    }
}

impl DoDVerifier for AutomlPipelineVerifier {
    fn verify(&self, _working_dir: &Path) -> Result<()> {
        let report = self.report()?;
        if report.is_pass() {
            Ok(())
        } else {
            Err(anyhow!(
                "AutoML DoD failed: {} plans failed, summary_balanced={}, banned strategy present={}",
                report.plans_failed, report.summary_json_balanced, report.config_has_ensemble_only,
            ))
        }
    }
}

/// Validate a single plan JSON file against all required invariants.
pub fn verify_plan_file(path: &Path) -> PlanCheck {
    let mut failures = Vec::new();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            failures.push(format!("read error: {}", e));
            return PlanCheck {
                path: path.to_path_buf(),
                passed: false,
                failures,
            };
        }
    };
    let plan: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            failures.push(format!("JSON parse error: {}", e));
            return PlanCheck {
                path: path.to_path_buf(),
                passed: false,
                failures,
            };
        }
    };

    // Required fields
    for field in &[
        "log",
        "log_idx",
        "fusion",
        "selected",
        "tiers",
        "plan_accuracy_vs_anchor",
        "plan_accuracy_vs_gt",
        "anchor_vs_gt",
        "oracle_signal",
        "oracle_vs_gt",
        "oracle_gap",
        "per_signal_gt_accuracy",
        "total_timing_us",
        "signals_evaluated",
        "signals_rejected_correlation",
        "signals_rejected_no_gain",
        "accounting_balanced",
        "pareto_front",
    ] {
        if plan.get(*field).is_none() {
            failures.push(format!("missing field: {}", field));
        }
    }

    // accounting_balanced == true
    if plan.get("accounting_balanced").and_then(|v| v.as_bool()) != Some(true) {
        failures.push("accounting_balanced is not true".to_string());
    }

    // selected.len() + rej_corr + rej_gain == evaluated (cross-check)
    let sel = plan
        .get("selected")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let rc = plan
        .get("signals_rejected_correlation")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let rg = plan
        .get("signals_rejected_no_gain")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let ev = plan
        .get("signals_evaluated")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    if sel + rc + rg != ev {
        failures.push(format!(
            "accounting identity broken: {} + {} + {} = {} ≠ evaluated={}",
            sel,
            rc,
            rg,
            sel + rc + rg,
            ev,
        ));
    }

    // pareto_front must have exactly one chosen=true
    if let Some(pf) = plan.get("pareto_front").and_then(|v| v.as_array()) {
        let chosen_count = pf
            .iter()
            .filter(|c| c.get("chosen").and_then(|v| v.as_bool()) == Some(true))
            .count();
        if chosen_count != 1 {
            failures.push(format!(
                "pareto_front has {} chosen=true candidates, expected exactly 1",
                chosen_count
            ));
        }
    }

    // oracle_gap must equal plan_accuracy_vs_gt - oracle_vs_gt
    let plan_vs_gt = plan.get("plan_accuracy_vs_gt").and_then(|v| v.as_f64());
    let oracle_vs_gt = plan.get("oracle_vs_gt").and_then(|v| v.as_f64());
    let oracle_gap = plan.get("oracle_gap").and_then(|v| v.as_f64());
    if let (Some(p), Some(o), Some(g)) = (plan_vs_gt, oracle_vs_gt, oracle_gap) {
        if (g - (p - o)).abs() > 1e-6 {
            failures.push(format!("oracle_gap lie: stored={} computed={}", g, p - o));
        }
    }

    let passed = failures.is_empty();
    PlanCheck {
        path: path.to_path_buf(),
        passed,
        failures,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// DX / QoL pipeline verifier
// ────────────────────────────────────────────────────────────────────────────

const EXPECTED_STRATEGIES: &[&str] = &[
    "f",
    "g",
    "h",
    "hdc",
    "automl",
    "rl_automl",
    "combinator",
    "sup_trained",
    "automl_hyper",
    "borda",
    "rrf",
    "weighted",
    "prec_weighted",
    "stacked",
    "full_combo",
    "best_pair",
    "combo",
    "vote500",
    "s_ensemble",
    "best_per_log",
];

/// Outcome of a DX / QoL definition-of-done check.
#[derive(Debug, Clone, Default)]
pub struct DxQolDodReport {
    /// `strategy_accuracies.json` exists and all 20 strategy keys are present.
    pub strategy_accuracies_ok: bool,
    /// `run_metadata.json` exists and has git_commit + timestamp + counts.
    pub run_metadata_ok: bool,
    /// At least one classified `.xes` output exists in the artifacts dir.
    pub output_xes_present: bool,
    /// Fraction of logs skipped (failed_xes + failed_gt) / n_logs_total.
    pub skip_rate: f64,
    /// All strategy accuracies are in [0, 1].
    pub accuracies_in_range: bool,
    /// best_per_log >= every individual strategy accuracy.
    pub best_per_log_dominates: bool,
    /// Collected failure messages.
    pub failures: Vec<String>,
}

impl DxQolDodReport {
    pub fn is_pass(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Verifies DX / QoL artifacts written by the PDC 2025 pipeline:
/// - `strategy_accuracies.json` contains all 20 strategy keys with values in [0,1]
/// - `run_metadata.json` is present with git commit, timestamp, and counts
/// - At least one output XES was written
/// - Skip rate < 10 %
/// - best_per_log dominates all individual strategies (correctness invariant)
pub struct DxQolVerifier {
    pub artifacts_dir: PathBuf,
    /// Maximum tolerable fraction of logs that may be skipped (default 0.10).
    pub max_skip_rate: f64,
}

impl DxQolVerifier {
    pub fn new(working_dir: &Path) -> Self {
        Self {
            artifacts_dir: working_dir.join("artifacts/pdc2025"),
            max_skip_rate: 0.10,
        }
    }

    pub fn report(&self) -> Result<DxQolDodReport> {
        let mut r = DxQolDodReport::default();

        // ── strategy_accuracies.json ──────────────────────────────────────
        let acc_path = self.artifacts_dir.join("strategy_accuracies.json");
        if !acc_path.exists() {
            r.failures.push("strategy_accuracies.json missing".into());
        } else {
            let content = std::fs::read_to_string(&acc_path)
                .with_context(|| format!("reading {:?}", acc_path))?;
            match serde_json::from_str::<serde_json::Value>(&content) {
                Err(e) => r
                    .failures
                    .push(format!("strategy_accuracies.json parse error: {e}")),
                Ok(v) => {
                    let strategies = v.get("strategies");
                    let mut all_present = true;
                    let mut all_in_range = true;
                    let mut best_per_log = 0.0_f64;
                    let mut max_individual = 0.0_f64;

                    for key in EXPECTED_STRATEGIES {
                        match strategies
                            .and_then(|s| s.get(*key))
                            .and_then(|v| v.as_f64())
                        {
                            None => {
                                r.failures
                                    .push(format!("strategy_accuracies.json: missing key '{key}'"));
                                all_present = false;
                            }
                            Some(acc) => {
                                if !(0.0..=1.0).contains(&acc) {
                                    r.failures.push(format!(
                                        "strategy_accuracies.json: '{key}' = {acc:.4} outside [0,1]"
                                    ));
                                    all_in_range = false;
                                }
                                if *key == "best_per_log" {
                                    best_per_log = acc;
                                } else {
                                    max_individual = max_individual.max(acc);
                                }
                            }
                        }
                    }

                    r.strategy_accuracies_ok = all_present;
                    r.accuracies_in_range = all_in_range;

                    // best_per_log must be >= every individual strategy (by construction)
                    r.best_per_log_dominates = best_per_log + 1e-9 >= max_individual;
                    if !r.best_per_log_dominates {
                        r.failures.push(format!(
                            "best_per_log ({best_per_log:.4}) < best individual ({max_individual:.4}) — lie detected"
                        ));
                    }

                    let n_logs = v.get("n_logs").and_then(|v| v.as_u64()).unwrap_or(0);
                    if n_logs == 0 {
                        r.failures.push(
                            "strategy_accuracies.json: n_logs = 0 — no logs processed".into(),
                        );
                    }
                }
            }
        }

        // ── run_metadata.json ─────────────────────────────────────────────
        let meta_path = self.artifacts_dir.join("run_metadata.json");
        if !meta_path.exists() {
            r.failures.push("run_metadata.json missing".into());
        } else {
            let content = std::fs::read_to_string(&meta_path)
                .with_context(|| format!("reading {:?}", meta_path))?;
            match serde_json::from_str::<serde_json::Value>(&content) {
                Err(e) => r
                    .failures
                    .push(format!("run_metadata.json parse error: {e}")),
                Ok(v) => {
                    for field in &[
                        "git_commit",
                        "timestamp",
                        "n_logs_total",
                        "n_logs_processed",
                        "n_failed_xes",
                        "n_failed_gt",
                    ] {
                        if v.get(*field).is_none() {
                            r.failures
                                .push(format!("run_metadata.json: missing field '{field}'"));
                        }
                    }

                    let total = v.get("n_logs_total").and_then(|v| v.as_u64()).unwrap_or(0) as f64;
                    let failed_xes =
                        v.get("n_failed_xes").and_then(|v| v.as_u64()).unwrap_or(0) as f64;
                    let failed_gt =
                        v.get("n_failed_gt").and_then(|v| v.as_u64()).unwrap_or(0) as f64;
                    r.skip_rate = if total > 0.0 {
                        (failed_xes + failed_gt) / total
                    } else {
                        0.0
                    };
                    if r.skip_rate > self.max_skip_rate {
                        r.failures.push(format!(
                            "skip rate {:.1}% exceeds threshold {:.1}%",
                            r.skip_rate * 100.0,
                            self.max_skip_rate * 100.0,
                        ));
                    }

                    if v.get("git_commit")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "unknown")
                        .unwrap_or(false)
                    {
                        // Warn only — git may legitimately be absent in CI
                        r.failures.push("run_metadata.json: git_commit = 'unknown' (not in a git repo or git not on PATH)".into());
                    }

                    r.run_metadata_ok = r.failures.iter().all(|f| !f.contains("run_metadata"));
                }
            }
        }

        // ── at least one output XES ───────────────────────────────────────
        r.output_xes_present = std::fs::read_dir(&self.artifacts_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().map(|x| x == "xes").unwrap_or(false))
            })
            .unwrap_or(false);
        if !r.output_xes_present {
            r.failures
                .push("no classified .xes files found in artifacts/pdc2025/".into());
        }

        Ok(r)
    }
}

impl DoDVerifier for DxQolVerifier {
    fn verify(&self, _working_dir: &Path) -> Result<()> {
        let report = self.report()?;
        if report.is_pass() {
            Ok(())
        } else {
            Err(anyhow!(
                "DX/QoL DoD failed ({} issue{}):\n{}",
                report.failures.len(),
                if report.failures.len() == 1 { "" } else { "s" },
                report
                    .failures
                    .iter()
                    .map(|f| format!("  • {f}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ))
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod dx_qol_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    fn make_valid_acc_json(n_logs: usize) -> String {
        let strats: String = EXPECTED_STRATEGIES
            .iter()
            .map(|k| format!("\"{k}\": 0.75"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{\"n_logs\": {n_logs}, \"strategies\": {{{strats}}}}}")
    }

    fn make_valid_meta_json() -> String {
        r#"{"git_commit":"abc1234","timestamp":1714000000,"n_logs_total":3,"n_logs_processed":3,"n_failed_xes":0,"n_failed_gt":0,"stem_filter":null}"#.into()
    }

    #[test]
    fn test_dxqol_pass_with_valid_artifacts() {
        let tmp = TempDir::new().unwrap();
        let arts = tmp.path().join("artifacts/pdc2025");
        fs::create_dir_all(&arts).unwrap();

        write_file(&arts, "strategy_accuracies.json", &make_valid_acc_json(3));
        write_file(&arts, "run_metadata.json", &make_valid_meta_json());
        fs::write(arts.join("pdc2025_000000.xes"), "<xes/>").unwrap();

        let v = DxQolVerifier {
            artifacts_dir: arts,
            max_skip_rate: 0.10,
        };
        let r = v.report().unwrap();
        assert!(r.is_pass(), "failures: {:?}", r.failures);
    }

    #[test]
    fn test_dxqol_fails_missing_strategy_key() {
        let tmp = TempDir::new().unwrap();
        let arts = tmp.path().join("artifacts/pdc2025");
        fs::create_dir_all(&arts).unwrap();

        // Missing "hdc" key
        let partial = r#"{"n_logs":3,"strategies":{"f":0.7,"g":0.7,"h":0.7}}"#;
        write_file(&arts, "strategy_accuracies.json", partial);
        write_file(&arts, "run_metadata.json", &make_valid_meta_json());
        fs::write(arts.join("out.xes"), "<xes/>").unwrap();

        let v = DxQolVerifier {
            artifacts_dir: arts,
            max_skip_rate: 0.10,
        };
        let r = v.report().unwrap();
        assert!(!r.is_pass());
        assert!(r.failures.iter().any(|f| f.contains("hdc")));
    }

    #[test]
    fn test_dxqol_fails_high_skip_rate() {
        let tmp = TempDir::new().unwrap();
        let arts = tmp.path().join("artifacts/pdc2025");
        fs::create_dir_all(&arts).unwrap();

        write_file(&arts, "strategy_accuracies.json", &make_valid_acc_json(10));
        let meta = r#"{"git_commit":"abc","timestamp":1,"n_logs_total":10,"n_logs_processed":5,"n_failed_xes":6,"n_failed_gt":0,"stem_filter":null}"#;
        write_file(&arts, "run_metadata.json", meta);
        fs::write(arts.join("out.xes"), "<xes/>").unwrap();

        let v = DxQolVerifier {
            artifacts_dir: arts,
            max_skip_rate: 0.10,
        };
        let r = v.report().unwrap();
        assert!(!r.is_pass());
        assert!(r.failures.iter().any(|f| f.contains("skip rate")));
    }

    #[test]
    fn test_dxqol_fails_no_output_xes() {
        let tmp = TempDir::new().unwrap();
        let arts = tmp.path().join("artifacts/pdc2025");
        fs::create_dir_all(&arts).unwrap();

        write_file(&arts, "strategy_accuracies.json", &make_valid_acc_json(3));
        write_file(&arts, "run_metadata.json", &make_valid_meta_json());
        // No .xes files written

        let v = DxQolVerifier {
            artifacts_dir: arts,
            max_skip_rate: 0.10,
        };
        let r = v.report().unwrap();
        assert!(!r.is_pass());
        assert!(r.failures.iter().any(|f| f.contains(".xes")));
    }

    #[test]
    fn test_dxqol_fails_accuracy_out_of_range() {
        let tmp = TempDir::new().unwrap();
        let arts = tmp.path().join("artifacts/pdc2025");
        fs::create_dir_all(&arts).unwrap();

        // "f" has value 1.5 — outside [0, 1]
        let bad_acc = make_valid_acc_json(3).replace("\"f\": 0.75", "\"f\": 1.5");
        write_file(&arts, "strategy_accuracies.json", &bad_acc);
        write_file(&arts, "run_metadata.json", &make_valid_meta_json());
        fs::write(arts.join("out.xes"), "<xes/>").unwrap();

        let v = DxQolVerifier {
            artifacts_dir: arts,
            max_skip_rate: 0.10,
        };
        let r = v.report().unwrap();
        assert!(!r.is_pass());
        assert!(r.failures.iter().any(|f| f.contains("outside [0,1]")));
    }
}
