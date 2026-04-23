//! Definition of Done — DX/QoL verifier for the AutoML/HDIT/TPOT2 pipeline.
//!
//! Runs all invariant checks and prints a clear pass/fail summary with
//! remediation hints. Intended for pre-commit, pre-merge, or post-smoke-test.
//!
//! Exit codes:
//!   0 — all checks pass
//!   1 — soft failure (build/tests OK but pipeline artifacts missing/stale)
//!   2 — hard failure (build broken, tests broken, or invariant violated)
//!
//! Usage: `cargo run --bin dod` or `cargo make dod`

use std::path::PathBuf;
use std::process::{Command, ExitCode};

use dteam::agentic::ralph::verifier::{AutomlPipelineVerifier, DxQolVerifier};

// ANSI color codes — no crate dependency; trivial to strip if needed.
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn check(name: &str, ok: bool, detail: &str) -> bool {
    if ok {
        println!("  {}✓{} {:<40} {}", GREEN, RESET, name, DIM);
        if !detail.is_empty() {
            println!("    {}{}{}", DIM, detail, RESET);
        }
    } else {
        println!(
            "  {}✗{} {:<40} {}{}{}",
            RED, RESET, name, RED, detail, RESET
        );
    }
    ok
}

fn warn(name: &str, detail: &str) {
    println!(
        "  {}⚠{} {:<40} {}{}{}",
        YELLOW, RESET, name, YELLOW, detail, RESET
    );
}

fn section(title: &str) {
    println!();
    println!(
        "{}{}── {} ──────────────────────────────────────────{}",
        BOLD, DIM, title, RESET
    );
}

fn main() -> ExitCode {
    let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    println!(
        "{}{}┌─────────────────────────────────────────────────────────────┐{}",
        BOLD, DIM, RESET
    );
    println!(
        "{}{}│ dteam Definition of Done — AutoML / HDIT / TPOT2 pipeline │{}",
        BOLD, DIM, RESET
    );
    println!(
        "{}{}└─────────────────────────────────────────────────────────────┘{}",
        BOLD, DIM, RESET
    );
    println!("  working dir: {}{}{}", DIM, working_dir.display(), RESET);

    let mut all_ok = true;
    let mut hard_fail = false;

    // ── Phase 1: Compilation ───────────────────────────────────────────────
    section("Phase 1: Compilation");
    let check_out = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(&working_dir)
        .output();
    match check_out {
        Ok(o) if o.status.success() => {
            check("cargo check", true, "clean");
        }
        Ok(o) => {
            all_ok = false;
            hard_fail = true;
            let stderr = String::from_utf8_lossy(&o.stderr);
            let first_err = stderr
                .lines()
                .find(|l| l.contains("error"))
                .unwrap_or("see `cargo check` output");
            check("cargo check", false, first_err);
        }
        Err(e) => {
            all_ok = false;
            hard_fail = true;
            check("cargo check", false, &format!("spawn failed: {}", e));
        }
    }

    // ── Phase 2: Tests ─────────────────────────────────────────────────────
    // Verbose output (no --quiet) so we can parse "test result: ok. N passed" lines.
    section("Phase 2: Tests");
    let test_out = Command::new("cargo")
        .args(["test", "--lib"])
        .current_dir(&working_dir)
        .output();
    match test_out {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Sum "N passed" across all test-result lines (may be several per-crate)
            let total_passed: usize = stdout
                .lines()
                .filter(|l| l.contains("test result: ok"))
                .filter_map(|l| {
                    let parts: Vec<&str> = l.split_whitespace().collect();
                    parts.iter().position(|&p| p == "passed;").and_then(|i| {
                        i.checked_sub(1)
                            .and_then(|j| parts.get(j))
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                })
                .sum();
            check(
                "cargo test --lib",
                true,
                &format!("{} tests passed", total_passed),
            );
        }
        Ok(o) => {
            all_ok = false;
            hard_fail = true;
            let stdout = String::from_utf8_lossy(&o.stdout);
            let failed_line = stdout
                .lines()
                .find(|l| l.contains("FAILED") || l.contains("failures"))
                .unwrap_or("see `cargo test --lib` output");
            check("cargo test --lib", false, failed_line);
        }
        Err(e) => {
            all_ok = false;
            hard_fail = true;
            check("cargo test --lib", false, &format!("spawn failed: {}", e));
        }
    }

    // ── Phase 3: Pipeline artifacts ────────────────────────────────────────
    section("Phase 3: Pipeline artifacts (AutoML plans)");
    let verifier = AutomlPipelineVerifier::new(&working_dir);
    let report = match verifier.report() {
        Ok(r) => r,
        Err(e) => {
            check("pipeline artifact scan", false, &format!("{}", e));
            return ExitCode::from(if hard_fail { 2 } else { 1 });
        }
    };

    // Config check
    check(
        "config has no banned ensemble_only strategy",
        !report.config_has_ensemble_only,
        if report.config_has_ensemble_only {
            "dteam.toml contains strategy=\"ensemble_only\" — this was removed as a no-op. Change to \"random\" or \"grid\"."
        } else {
            ""
        },
    );
    if report.config_has_ensemble_only {
        all_ok = false;
    }

    // Plan artifacts
    if report.plans_checked == 0 {
        warn(
            "no plan JSON files found",
            "run `cargo run --bin pdc2025 --release` with [automl].enabled=true to generate plans",
        );
    } else {
        let plans_ok = report.plans_failed == 0;
        check(
            &format!("plan JSON invariants ({} plans)", report.plans_checked),
            plans_ok,
            if plans_ok {
                "all plans balanced, exactly 1 chosen Pareto candidate each, all required fields present"
            } else {
                "see per-plan failures below"
            },
        );
        if !plans_ok {
            all_ok = false;
            for failed in &report.failed_plans {
                println!(
                    "    {}└─ {}{}",
                    DIM,
                    failed.path.file_name().unwrap().to_string_lossy(),
                    RESET
                );
                for f in &failed.failures {
                    println!("       {}• {}{}", RED, f, RESET);
                }
            }
        }
    }

    // Summary JSON
    if report.plans_checked > 0 {
        check(
            "automl_summary.json exists",
            report.summary_json_present,
            if report.summary_json_present {
                "written from disk roundtrip"
            } else {
                "missing — did the binary complete?"
            },
        );
        if !report.summary_json_present {
            all_ok = false;
        }

        if report.summary_json_present {
            check(
                "summary accounting_balanced_global=true",
                report.summary_json_balanced,
                if report.summary_json_balanced {
                    "global accounting identity holds"
                } else {
                    "INVARIANT VIOLATION — aggregate is a lie"
                },
            );
            if !report.summary_json_balanced {
                all_ok = false;
            }
        }
    }

    // ── Phase 4: DX / QoL artifacts ───────────────────────────────────────
    section("Phase 4: DX / QoL artifacts");
    let dxqol = DxQolVerifier::new(&working_dir);
    match dxqol.report() {
        Err(e) => {
            all_ok = false;
            check("DX/QoL artifact scan", false, &format!("{e}"));
        }
        Ok(dq) => {
            let acc_ok = check(
                "strategy_accuracies.json",
                dq.strategy_accuracies_ok && dq.accuracies_in_range,
                if dq.strategy_accuracies_ok && dq.accuracies_in_range {
                    "all 20 strategies present, values in [0,1]"
                } else {
                    "missing keys or out-of-range values — run `cargo make pdc`"
                },
            );
            let meta_ok = check(
                "run_metadata.json",
                dq.run_metadata_ok,
                if dq.run_metadata_ok {
                    "git_commit + timestamp + counts present"
                } else {
                    "missing or incomplete — run `cargo make pdc`"
                },
            );
            let xes_ok = check(
                "classified output XES files",
                dq.output_xes_present,
                if dq.output_xes_present {
                    "at least one .xes written"
                } else {
                    "no output XES found — run `cargo make pdc`"
                },
            );
            let skip_ok = dq.skip_rate <= dxqol.max_skip_rate;
            check(
                &format!("skip rate ≤ {:.0}%", dxqol.max_skip_rate * 100.0),
                skip_ok,
                &format!("actual {:.1}%", dq.skip_rate * 100.0),
            );
            let best_ok = check(
                "best_per_log dominates all individual strategies",
                dq.best_per_log_dominates || !dq.strategy_accuracies_ok,
                if dq.best_per_log_dominates {
                    "invariant holds"
                } else {
                    "best_per_log < best individual — lie detected"
                },
            );

            if !acc_ok || !meta_ok || !xes_ok || !skip_ok || !best_ok {
                all_ok = false;
                for f in &dq.failures {
                    println!("    {}• {}{}", RED, f, RESET);
                }
            }
        }
    }

    // ── Verdict ────────────────────────────────────────────────────────────
    section("Verdict");
    if all_ok {
        println!("  {}{}✓ Definition of Done: PASS{}", BOLD, GREEN, RESET);
        println!(
            "  {}compile ✓  tests ✓  {} plans ✓  DX/QoL artifacts ✓{}",
            DIM, report.plans_checked, RESET
        );
        ExitCode::from(0)
    } else if hard_fail {
        println!("  {}{}✗ Definition of Done: HARD FAIL{}", BOLD, RED, RESET);
        println!(
            "  {}compilation or tests broken — fix before anything else{}",
            DIM, RESET
        );
        ExitCode::from(2)
    } else {
        println!(
            "  {}{}⚠ Definition of Done: SOFT FAIL{}",
            BOLD, YELLOW, RESET
        );
        println!(
            "  {}build+tests OK but pipeline artifacts have issues{}",
            DIM, RESET
        );
        ExitCode::from(1)
    }
}
