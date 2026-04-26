//! dteam doctor — epistemic diagnostic for AutoML / ensemble pipeline artifacts.
//!
//! Answers: "is your plan slow, redundant, biased, or lying?"
//! Modeled after `brew doctor` / `flutter doctor` but performs epistemic
//! diagnosis of plan JSON artifacts rather than environment checks.
//!
//! # Exit codes
//!   0 — all checks pass (healthy)
//!   1 — soft fail (SLOW / REDUNDANT / SATURATED / STALE — suboptimal, not lying)
//!   2 — fatal (LYING — invariant violated)
//!
//! # Usage
//!   cargo run --bin doctor
//!   cargo run --bin doctor -- --target=T1
//!   cargo run --bin doctor -- --plan=artifacts/pdc2025/automl_plans/pdc2025_000000.json
//!   cargo run --bin doctor -- --json
//!   cargo run --bin doctor -- --plans-dir=path/to/plans

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

// ── ANSI color codes — no crate dependency ────────────────────────────────────
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// ── Tier thresholds (µs) ──────────────────────────────────────────────────────
const T0_MAX_US: u64 = 100;
const T1_MAX_US: u64 = 2_000;
const T2_MAX_US: u64 = 100_000;

// ── Signal family classification ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SignalFamily {
    Conformance,
    NlpSymbolic,
    HyperdimSynthetic,
    AutomlRl,
    MlEnsemble,
}

impl SignalFamily {
    fn name(&self) -> &'static str {
        match self {
            Self::Conformance => "conformance",
            Self::NlpSymbolic => "nlp_symbolic",
            Self::HyperdimSynthetic => "hyperdim_synthetic",
            Self::AutomlRl => "automl_rl",
            Self::MlEnsemble => "ml_ensemble",
        }
    }
}

fn classify_signal(signal: &str) -> SignalFamily {
    let s = signal.to_lowercase();
    // Conformance family
    if matches!(
        s.as_str(),
        "h_inlang_fill" | "g_fitness_rank" | "f_classify_exact"
    ) || s.contains("fitness")
        || s.contains("inlang")
        || s.contains("classify_exact")
    {
        return SignalFamily::Conformance;
    }
    // NLP/symbolic family
    if matches!(s.as_str(), "tf_idf" | "ngram" | "pagerank" | "e_edit_dist")
        || s.contains("tfidf")
        || s.contains("tf_idf")
        || s.contains("ngram")
        || s.contains("pagerank")
        || s.contains("edit")
    {
        return SignalFamily::NlpSymbolic;
    }
    // Hyperdimensional/synthetic family
    if matches!(s.as_str(), "hdc_prototype" | "s_synthetic")
        || s.contains("hdc")
        || s.contains("synthetic")
    {
        return SignalFamily::HyperdimSynthetic;
    }
    // AutoML/RL family
    if s.contains("rl_")
        || s.contains("automl")
        || s.contains("combinator")
        || s.contains("rl_automl")
        || s.contains("automl_hyper")
    {
        return SignalFamily::AutomlRl;
    }
    // Everything else
    SignalFamily::MlEnsemble
}

// ── Tier classification ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tier {
    T0,
    T1,
    T2,
    Warm,
}

impl Tier {
    fn from_us(us: u64) -> Self {
        if us <= T0_MAX_US {
            Self::T0
        } else if us <= T1_MAX_US {
            Self::T1
        } else if us <= T2_MAX_US {
            Self::T2
        } else {
            Self::Warm
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::T0 => "T0",
            Self::T1 => "T1",
            Self::T2 => "T2",
            Self::Warm => "Warm",
        }
    }

    fn deployment(&self) -> &'static str {
        match self {
            Self::T0 => "browser/WASM, embedded, real-time",
            Self::T1 => "edge/CDN (Cloudflare Workers, Fastly), mobile on-device",
            Self::T2 => "fog/serverless (Lambda, Cloud Run), IoT gateway",
            Self::Warm => "cloud (EC2, GKE), batch, offline",
        }
    }

    fn max_us(&self) -> Option<u64> {
        match self {
            Self::T0 => Some(T0_MAX_US),
            Self::T1 => Some(T1_MAX_US),
            Self::T2 => Some(T2_MAX_US),
            Self::Warm => None,
        }
    }
}

// ── Plan data ─────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct ParetoCandidate {
    signals: Vec<String>,
    total_timing_us: u64,
    accuracy_vs_anchor: f64,
    chosen: bool,
}

#[derive(Debug)]
struct Plan {
    #[allow(dead_code)]
    path: PathBuf,
    log: String,
    accounting_balanced: bool,
    selected: Vec<String>,
    signals_evaluated: usize,
    signals_rejected_correlation: usize,
    signals_rejected_no_gain: usize,
    total_timing_us: u64,
    /// (signal_name, tier_label) pairs from the `tiers` array
    tiers: Vec<(String, String)>,
    pareto_front: Vec<ParetoCandidate>,
    plan_accuracy_vs_gt: f64,
    oracle_vs_gt: f64,
    oracle_gap: f64,
    parse_error: Option<String>,
}

impl Plan {
    fn load(path: &Path) -> Self {
        let mut plan = Plan {
            path: path.to_path_buf(),
            log: path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            accounting_balanced: false,
            selected: vec![],
            signals_evaluated: 0,
            signals_rejected_correlation: 0,
            signals_rejected_no_gain: 0,
            total_timing_us: 0,
            tiers: vec![],
            pareto_front: vec![],
            plan_accuracy_vs_gt: 0.0,
            oracle_vs_gt: 0.0,
            oracle_gap: f64::NAN,
            parse_error: None,
        };

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                plan.parse_error = Some(format!("read error: {e}"));
                return plan;
            }
        };

        let v: Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                plan.parse_error = Some(format!("JSON parse error: {e}"));
                return plan;
            }
        };

        plan.log = v
            .get("log")
            .and_then(|x| x.as_str())
            .unwrap_or(&plan.log)
            .to_string();
        plan.accounting_balanced = v
            .get("accounting_balanced")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        plan.selected = v
            .get("selected")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        plan.signals_evaluated = v
            .get("signals_evaluated")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        plan.signals_rejected_correlation = v
            .get("signals_rejected_correlation")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        plan.signals_rejected_no_gain = v
            .get("signals_rejected_no_gain")
            .and_then(|x| x.as_u64())
            .unwrap_or(0) as usize;
        plan.total_timing_us = v
            .get("total_timing_us")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        plan.tiers = v
            .get("tiers")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|t| {
                        let sig = t.get("signal")?.as_str()?.to_string();
                        let tier = t.get("tier")?.as_str()?.to_string();
                        Some((sig, tier))
                    })
                    .collect()
            })
            .unwrap_or_default();
        plan.pareto_front = v
            .get("pareto_front")
            .and_then(|x| x.as_array())
            .map(|a| {
                a.iter()
                    .map(|c| ParetoCandidate {
                        signals: c
                            .get("signals")
                            .and_then(|s| s.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|s| s.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        total_timing_us: c
                            .get("total_timing_us")
                            .and_then(|x| x.as_u64())
                            .unwrap_or(0),
                        accuracy_vs_anchor: c
                            .get("accuracy_vs_anchor")
                            .and_then(|x| x.as_f64())
                            .unwrap_or(0.0),
                        chosen: c.get("chosen").and_then(|x| x.as_bool()).unwrap_or(false),
                    })
                    .collect()
            })
            .unwrap_or_default();
        plan.plan_accuracy_vs_gt = v
            .get("plan_accuracy_vs_gt")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        plan.oracle_vs_gt = v
            .get("oracle_vs_gt")
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        plan.oracle_gap = v
            .get("oracle_gap")
            .and_then(|x| x.as_f64())
            .unwrap_or(f64::NAN);

        plan
    }

    fn tier(&self) -> Tier {
        Tier::from_us(self.total_timing_us)
    }
}

// ── Output helpers (follow dod.rs style exactly) ──────────────────────────────

fn check(name: &str, ok: bool, detail: &str) -> bool {
    if ok {
        println!(
            "  {}✓{} {:<42} {}{}{}",
            GREEN, RESET, name, DIM, detail, RESET
        );
    } else {
        println!(
            "  {}✗{} {:<42} {}{}{}",
            RED, RESET, name, RED, detail, RESET
        );
    }
    ok
}

fn warn_line(name: &str, detail: &str) {
    println!(
        "  {}⚠{} {:<42} {}{}{}",
        YELLOW, RESET, name, YELLOW, detail, RESET
    );
}

fn info_line(name: &str, detail: &str) {
    println!(
        "  {}ℹ{} {:<42} {}{}{}",
        DIM, RESET, name, DIM, detail, RESET
    );
}

fn section(title: &str) {
    println!();
    println!(
        "{}{}── {} ─────────────────────────────────────────────────{}",
        BOLD, DIM, title, RESET
    );
}

fn fmt_us(us: u64) -> String {
    if us < 1_000 {
        format!("{}µs", us)
    } else if us < 1_000_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else {
        format!("{:.2}s", us as f64 / 1_000_000.0)
    }
}

// ── CLI arg parsing ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorKind {
    Automl,
    RalphPlan,
}

struct CliArgs {
    target: Option<String>,
    plan: Option<PathBuf>,
    json: bool,
    plans_dir: Option<PathBuf>,
    kind: DoctorKind,
}

fn parse_args() -> CliArgs {
    let mut args = CliArgs {
        target: None,
        plan: None,
        json: false,
        plans_dir: None,
        kind: DoctorKind::Automl,
    };
    for raw in std::env::args().skip(1) {
        if raw == "--json" {
            args.json = true;
        } else if let Some(val) = raw.strip_prefix("--target=") {
            args.target = Some(val.to_string());
        } else if let Some(val) = raw.strip_prefix("--plan=") {
            args.plan = Some(PathBuf::from(val));
        } else if let Some(val) = raw.strip_prefix("--plans-dir=") {
            args.plans_dir = Some(PathBuf::from(val));
        } else if let Some(val) = raw.strip_prefix("--kind=") {
            args.kind = match val {
                "ralph-plan" | "ralph_plan" => DoctorKind::RalphPlan,
                "automl" | "" => DoctorKind::Automl,
                other => {
                    eprintln!("doctor: unknown --kind={} (allowed: automl, ralph-plan)", other);
                    std::process::exit(2);
                }
            };
        }
    }
    args
}

// ── Plan loading ──────────────────────────────────────────────────────────────

fn load_plans(plans_dir: &Path) -> Vec<Plan> {
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(plans_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
            .collect(),
        Err(_) => return vec![],
    };
    paths.sort();
    paths.iter().map(|p| Plan::load(p)).collect()
}

// ── Pathology: LYING ──────────────────────────────────────────────────────────

struct TruthReport {
    unbalanced: Vec<String>,
    accounting_broken: Vec<String>,
    pareto_invariant_broken: Vec<String>,
    oracle_gap_lies: Vec<String>,
    parse_errors: Vec<String>,
}

fn check_truth(plans: &[Plan]) -> TruthReport {
    let mut r = TruthReport {
        unbalanced: vec![],
        accounting_broken: vec![],
        pareto_invariant_broken: vec![],
        oracle_gap_lies: vec![],
        parse_errors: vec![],
    };
    for p in plans {
        if let Some(e) = &p.parse_error {
            r.parse_errors.push(format!("{}: {}", p.log, e));
            continue;
        }
        if !p.accounting_balanced {
            r.unbalanced.push(p.log.clone());
        }
        let sum = p.selected.len() + p.signals_rejected_correlation + p.signals_rejected_no_gain;
        if sum != p.signals_evaluated {
            r.accounting_broken.push(format!(
                "{}: {} + {} + {} = {} ≠ {}",
                p.log,
                p.selected.len(),
                p.signals_rejected_correlation,
                p.signals_rejected_no_gain,
                sum,
                p.signals_evaluated
            ));
        }
        let chosen_count = p.pareto_front.iter().filter(|c| c.chosen).count();
        if chosen_count != 1 {
            r.pareto_invariant_broken.push(format!(
                "{}: {} chosen candidates (expected 1)",
                p.log, chosen_count
            ));
        }
        if !p.oracle_gap.is_nan() {
            let expected = p.plan_accuracy_vs_gt - p.oracle_vs_gt;
            if (p.oracle_gap - expected).abs() > 1e-6 {
                r.oracle_gap_lies.push(format!(
                    "{}: stored {:.6} ≠ computed {:.6}",
                    p.log, p.oracle_gap, expected
                ));
            }
        }
    }
    r
}

// ── Pathology: SLOW ───────────────────────────────────────────────────────────

struct DowngradeOption {
    signals: Vec<String>,
    timing_us: u64,
    accuracy_vs_anchor: f64,
    acc_delta: f64,
}

struct SlowReport {
    warm_plans: Vec<String>,
    /// (signal_name, timing_us) for signals in warm-tier selected plans
    warm_signals: Vec<(String, u64)>,
    cheapest_t2_downgrade: Option<DowngradeOption>,
}

fn check_slow(plans: &[Plan]) -> SlowReport {
    let mut warm_plans: Vec<String> = vec![];
    // Track all signals seen in warm plans alongside their plan's total timing
    let mut warm_sig_map: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut best_downgrade: Option<DowngradeOption> = None;

    for plan in plans {
        if plan.parse_error.is_some() {
            continue;
        }
        if plan.total_timing_us > T2_MAX_US {
            warm_plans.push(plan.log.clone());

            // Record the selected signals with their plan's total timing
            for (sig, _tier_lbl) in &plan.tiers {
                let entry = warm_sig_map.entry(sig.clone()).or_insert(0u64);
                if plan.total_timing_us > *entry {
                    *entry = plan.total_timing_us;
                }
            }

            // Find cheapest T2-eligible candidate in Pareto front (non-chosen)
            let chosen_acc = plan.plan_accuracy_vs_gt;
            for candidate in &plan.pareto_front {
                if !candidate.chosen && candidate.total_timing_us <= T2_MAX_US {
                    let delta = candidate.accuracy_vs_anchor - chosen_acc;
                    let is_better = match &best_downgrade {
                        None => true,
                        Some(bd) => candidate.accuracy_vs_anchor > bd.accuracy_vs_anchor,
                    };
                    if is_better {
                        best_downgrade = Some(DowngradeOption {
                            signals: candidate.signals.clone(),
                            timing_us: candidate.total_timing_us,
                            accuracy_vs_anchor: candidate.accuracy_vs_anchor,
                            acc_delta: delta,
                        });
                    }
                }
            }
        }
    }

    let mut warm_signals: Vec<(String, u64)> = warm_sig_map.into_iter().collect();
    warm_signals.sort_by_key(|(_, us)| std::cmp::Reverse(*us));

    SlowReport {
        warm_plans,
        warm_signals,
        cheapest_t2_downgrade: best_downgrade,
    }
}

// ── Pathology: SATURATED ──────────────────────────────────────────────────────

struct SaturationReport {
    /// (log_name, family) for each saturated plan
    saturated_plans: Vec<(String, SignalFamily)>,
    /// The most common single-family across saturated plans
    dominant_family: Option<SignalFamily>,
}

fn check_saturation(plans: &[Plan]) -> SaturationReport {
    let mut saturated_plans: Vec<(String, SignalFamily)> = vec![];
    let mut family_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for plan in plans {
        if plan.parse_error.is_some() || plan.selected.is_empty() {
            continue;
        }
        let families: std::collections::HashSet<String> = plan
            .selected
            .iter()
            .map(|s| classify_signal(s).name().to_string())
            .collect();
        if families.len() == 1 {
            let fam = classify_signal(&plan.selected[0]);
            *family_counts.entry(fam.name().to_string()).or_insert(0) += 1;
            saturated_plans.push((plan.log.clone(), fam));
        }
    }

    let dominant_family = family_counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(name, _)| match name.as_str() {
            "conformance" => SignalFamily::Conformance,
            "nlp_symbolic" => SignalFamily::NlpSymbolic,
            "hyperdim_synthetic" => SignalFamily::HyperdimSynthetic,
            "automl_rl" => SignalFamily::AutomlRl,
            _ => SignalFamily::MlEnsemble,
        });

    SaturationReport {
        saturated_plans,
        dominant_family,
    }
}

// ── Pathology: REDUNDANT ──────────────────────────────────────────────────────

struct RedundancyReport {
    avg_correlation_rejected: f64,
    /// Plans where >40% of evaluated signals were rejected for correlation
    high_redundancy_plans: Vec<String>,
}

fn check_redundancy(plans: &[Plan]) -> RedundancyReport {
    let valid: Vec<&Plan> = plans.iter().filter(|p| p.parse_error.is_none()).collect();
    if valid.is_empty() {
        return RedundancyReport {
            avg_correlation_rejected: 0.0,
            high_redundancy_plans: vec![],
        };
    }
    let total_rej: f64 = valid
        .iter()
        .map(|p| p.signals_rejected_correlation as f64)
        .sum();
    let avg = total_rej / valid.len() as f64;

    let high_redundancy_plans: Vec<String> = valid
        .iter()
        .filter(|p| {
            p.signals_evaluated > 0
                && (p.signals_rejected_correlation as f64 / p.signals_evaluated as f64) > 0.4
        })
        .map(|p| p.log.clone())
        .collect();

    RedundancyReport {
        avg_correlation_rejected: avg,
        high_redundancy_plans,
    }
}

// ── Tier distribution ─────────────────────────────────────────────────────────

struct TierDist {
    t0: usize,
    t1: usize,
    t2: usize,
    warm: usize,
    total: usize,
}

fn tier_distribution(plans: &[Plan]) -> TierDist {
    let valid: Vec<&Plan> = plans.iter().filter(|p| p.parse_error.is_none()).collect();
    let total = valid.len();
    let t0 = valid.iter().filter(|p| p.tier() == Tier::T0).count();
    let t1 = valid.iter().filter(|p| p.tier() == Tier::T1).count();
    let t2 = valid.iter().filter(|p| p.tier() == Tier::T2).count();
    let warm = valid.iter().filter(|p| p.tier() == Tier::Warm).count();
    TierDist {
        t0,
        t1,
        t2,
        warm,
        total,
    }
}

// ── Git staleness ─────────────────────────────────────────────────────────────

fn get_git_head() -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        })
}

fn get_artifact_commit(artifacts_dir: &Path) -> Option<String> {
    let meta_path = artifacts_dir.join("run_metadata.json");
    let content = std::fs::read_to_string(meta_path).ok()?;
    let v: Value = serde_json::from_str(&content).ok()?;
    v.get("git_commit")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty() && *s != "unknown")
        .map(str::to_string)
}

// ── --target mode ─────────────────────────────────────────────────────────────

fn target_tier_from_str(s: &str) -> Option<Tier> {
    match s.to_lowercase().as_str() {
        "t0" => Some(Tier::T0),
        "t1" => Some(Tier::T1),
        "t2" => Some(Tier::T2),
        "warm" => Some(Tier::Warm),
        _ => None,
    }
}

fn run_target_mode(target: &Tier, plans: &[Plan]) {
    let valid: Vec<&Plan> = plans.iter().filter(|p| p.parse_error.is_none()).collect();
    let max_us = target.max_us().unwrap_or(u64::MAX);

    let qualifying_count = valid.iter().filter(|p| p.total_timing_us <= max_us).count();
    let blocked: Vec<&&Plan> = valid
        .iter()
        .filter(|p| p.total_timing_us > max_us)
        .collect();

    section(&format!(
        "Target: {} ({}, <{})",
        target.label(),
        target.deployment().split(',').next().unwrap_or(""),
        match target.max_us() {
            Some(u) if u < 1_000 => format!("{}µs", u),
            Some(u) => format!("{}ms", u / 1_000),
            None => "∞".to_string(),
        }
    ));

    println!(
        "  {}{}/{} plans qualify at {}{}",
        DIM,
        qualifying_count,
        valid.len(),
        target.label(),
        RESET
    );

    if !blocked.is_empty() {
        println!(
            "  {}blocking signals in remaining {}:{}",
            YELLOW,
            blocked.len(),
            RESET
        );
        // Collect blocking signals across blocked plans
        let mut blocking_sigs: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();
        for plan in &blocked {
            for (sig, _) in &plan.tiers {
                blocking_sigs
                    .entry(sig.clone())
                    .and_modify(|e| *e = (*e).max(plan.total_timing_us))
                    .or_insert(plan.total_timing_us);
            }
        }
        let mut blocking_sorted: Vec<(String, u64)> = blocking_sigs.into_iter().collect();
        blocking_sorted.sort_by_key(|(_, us)| std::cmp::Reverse(*us));
        for (sig, us) in blocking_sorted.iter().take(10) {
            let tier = Tier::from_us(*us);
            println!(
                "    {}{:<32} {}  → {}{}",
                DIM,
                sig,
                fmt_us(*us),
                tier.label(),
                RESET
            );
        }

        println!(
            "\n  {}cheapest {} plan per blocked log:{}",
            YELLOW,
            target.label(),
            RESET
        );
        for plan in blocked.iter().take(10) {
            let best = plan
                .pareto_front
                .iter()
                .filter(|c| !c.chosen && c.total_timing_us <= max_us)
                .max_by(|a, b| {
                    a.accuracy_vs_anchor
                        .partial_cmp(&b.accuracy_vs_anchor)
                        .unwrap()
                });
            if let Some(cand) = best {
                let delta = cand.accuracy_vs_anchor - plan.plan_accuracy_vs_gt;
                println!(
                    "    {}{}  drop to [{}] at {}, acc {:.3} (Δ {:.3}){}",
                    DIM,
                    plan.log,
                    cand.signals.join(", "),
                    fmt_us(cand.total_timing_us),
                    cand.accuracy_vs_anchor,
                    delta,
                    RESET
                );
            } else {
                println!(
                    "    {}{}  no {} candidate in Pareto front{}",
                    DIM,
                    plan.log,
                    target.label(),
                    RESET
                );
            }
        }
    }
}

// ── Single-plan deep-dive ─────────────────────────────────────────────────────

fn run_single_plan(plan_path: &Path) {
    let plan = Plan::load(plan_path);

    println!(
        "{}{}┌─────────────────────────────────────────────────────────────────────────┐{}",
        BOLD, DIM, RESET
    );
    println!(
        "{}{}│ dteam doctor — single plan deep-dive                                    │{}",
        BOLD, DIM, RESET
    );
    println!(
        "{}{}└─────────────────────────────────────────────────────────────────────────┘{}",
        BOLD, DIM, RESET
    );
    println!("  plan: {}{}{}", DIM, plan_path.display(), RESET);

    if let Some(e) = &plan.parse_error {
        println!("  {}✗ parse error: {}{}", RED, e, RESET);
        return;
    }

    println!("  log:  {}{}{}", DIM, plan.log, RESET);
    println!(
        "  tier: {}{} ({}){}",
        DIM,
        plan.tier().label(),
        fmt_us(plan.total_timing_us),
        RESET
    );

    section("Truth");
    check(
        "accounting_balanced",
        plan.accounting_balanced,
        if plan.accounting_balanced {
            "true"
        } else {
            "INVARIANT VIOLATED"
        },
    );
    let sum =
        plan.selected.len() + plan.signals_rejected_correlation + plan.signals_rejected_no_gain;
    check(
        "accounting identity",
        sum == plan.signals_evaluated,
        &format!(
            "{} + {} + {} = {} (evaluated {})",
            plan.selected.len(),
            plan.signals_rejected_correlation,
            plan.signals_rejected_no_gain,
            sum,
            plan.signals_evaluated
        ),
    );
    let chosen_count = plan.pareto_front.iter().filter(|c| c.chosen).count();
    check(
        "pareto chosen invariant",
        chosen_count == 1,
        &format!("{} chosen candidates", chosen_count),
    );
    let expected_gap = plan.plan_accuracy_vs_gt - plan.oracle_vs_gt;
    check(
        "oracle gap formula",
        !plan.oracle_gap.is_nan() && (plan.oracle_gap - expected_gap).abs() <= 1e-6,
        &format!(
            "stored {:.6}, computed {:.6}",
            plan.oracle_gap, expected_gap
        ),
    );

    section("Speed");
    let tier = plan.tier();
    println!(
        "  {}total timing:  {} ({}){}",
        DIM,
        fmt_us(plan.total_timing_us),
        tier.label(),
        RESET
    );
    println!("  {}deployment:    {}{}", DIM, tier.deployment(), RESET);
    for (sig, tier_lbl) in &plan.tiers {
        println!("    {}{:<32} {}{}", DIM, sig, tier_lbl, RESET);
    }

    section("Signals");
    println!(
        "  {}selected ({}) : {}{}",
        DIM,
        plan.selected.len(),
        plan.selected.join(", "),
        RESET
    );
    println!(
        "  {}rejected correlation: {}  no-gain: {}{}",
        DIM, plan.signals_rejected_correlation, plan.signals_rejected_no_gain, RESET
    );
    let families: std::collections::HashSet<String> = plan
        .selected
        .iter()
        .map(|s| classify_signal(s).name().to_string())
        .collect();
    let mut fam_vec: Vec<String> = families.into_iter().collect();
    fam_vec.sort();
    println!("  {}signal families: {}{}", DIM, fam_vec.join(", "), RESET);

    section("Pareto Front");
    for c in &plan.pareto_front {
        let marker = if c.chosen {
            format!("{}★ chosen{}", GREEN, RESET)
        } else {
            format!("{}  {}", DIM, RESET)
        };
        println!(
            "  {} [{:<22}]  {}  acc_vs_anchor={:.3}{}",
            marker,
            c.signals.join(","),
            fmt_us(c.total_timing_us),
            c.accuracy_vs_anchor,
            RESET
        );
    }
}

// ── JSON output ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn output_json(
    _plans: &[Plan],
    truth: &TruthReport,
    slow: &SlowReport,
    saturation: &SaturationReport,
    redundancy: &RedundancyReport,
    tiers: &TierDist,
    git_match: bool,
    artifact_commit: Option<&str>,
    head_commit: Option<&str>,
) {
    let mut pathologies: Vec<Value> = vec![];

    let lying_count = truth.unbalanced.len()
        + truth.accounting_broken.len()
        + truth.pareto_invariant_broken.len()
        + truth.oracle_gap_lies.len()
        + truth.parse_errors.len();

    if lying_count > 0 {
        pathologies.push(serde_json::json!({
            "name": "LYING",
            "severity": "fatal",
            "count": lying_count,
            "message": "Invariant violated — plan accounting or oracle gap is a lie",
            "repair": "Run `cargo make pdc` to regenerate plans from scratch"
        }));
    }
    if !slow.warm_plans.is_empty() {
        pathologies.push(serde_json::json!({
            "name": "SLOW",
            "severity": "warn",
            "count": slow.warm_plans.len(),
            "message": format!("{} plans exceed Warm tier (>100ms)", slow.warm_plans.len()),
            "repair": "Disable Combo_ensemble or Vote500 signals, or accept T2 downgrade from Pareto front"
        }));
    }
    if !saturation.saturated_plans.is_empty() {
        pathologies.push(serde_json::json!({
            "name": "SATURATED",
            "severity": "warn",
            "count": saturation.saturated_plans.len(),
            "message": format!("{}/{} plans use a single signal family", saturation.saturated_plans.len(), tiers.total),
            "repair": "Add TF-IDF (T2) or HDC (T2) for orthogonal signal coverage"
        }));
    }
    if !redundancy.high_redundancy_plans.is_empty() {
        pathologies.push(serde_json::json!({
            "name": "REDUNDANT",
            "severity": "info",
            "count": redundancy.high_redundancy_plans.len(),
            "message": ">40% of signals rejected for correlation",
            "repair": "Diversify signal pool to reduce correlated candidates"
        }));
    }
    if !git_match {
        pathologies.push(serde_json::json!({
            "name": "STALE",
            "severity": "info",
            "count": 1,
            "message": format!("artifact commit {} ≠ HEAD {}",
                artifact_commit.unwrap_or("?"),
                head_commit.unwrap_or("?")),
            "repair": "Run `cargo make pdc` to regenerate artifacts from current HEAD"
        }));
    }

    let verdict = if lying_count > 0 {
        "fatal"
    } else if !slow.warm_plans.is_empty() || !saturation.saturated_plans.is_empty() {
        "soft_fail"
    } else {
        "healthy"
    };

    let out = serde_json::json!({
        "plans_read": tiers.total,
        "pathologies": pathologies,
        "tier_distribution": {
            "t0": tiers.t0,
            "t1": tiers.t1,
            "t2": tiers.t2,
            "warm": tiers.warm
        },
        "deployment_matrix": {
            "browser_wasm": tiers.t0,
            "edge_cdn": tiers.t0 + tiers.t1,
            "fog_serverless": tiers.t0 + tiers.t1 + tiers.t2,
            "cloud": tiers.total
        },
        "git_commit_match": git_match,
        "verdict": verdict
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

// ── RalphPlan mode ────────────────────────────────────────────────────────────
//
// Distinct from AutomlPlan diagnosis: RalphPlan accounts for Spec Kit phase
// execution per processed idea. Pathologies classified here include schema
// mismatch, accounting unbalance, missing artifact for a completed phase, and
// false completion (a phase marked complete with a failing gate).

#[derive(Default)]
struct RalphPlanReport {
    plans_read: usize,
    schema_mismatch: Vec<String>,
    unbalanced: Vec<String>,
    false_completion: Vec<String>,
    missing_artifact: Vec<String>,
    blocked: Vec<String>,
    skipped: Vec<String>,
    soft_fail_count: usize,
    pass_count: usize,
    fatal_count: usize,
}

fn run_ralph_plan_mode(plans_dir: &Path, json_mode: bool) -> ExitCode {
    use dteam::ralph_plan::{GateStatus, RalphPlan, Verdict};

    if !plans_dir.exists() {
        if json_mode {
            let out = serde_json::json!({
                "kind": "ralph-plan",
                "error": format!("plans directory not found: {}", plans_dir.display()),
                "verdict": "fatal"
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        } else {
            eprintln!("ralph-plan plans directory not found: {}", plans_dir.display());
        }
        return ExitCode::from(1);
    }

    let mut report = RalphPlanReport::default();
    let entries: Vec<PathBuf> = match std::fs::read_dir(plans_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|x| x == "json").unwrap_or(false))
            .collect(),
        Err(_) => Vec::new(),
    };

    for path in &entries {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                report
                    .schema_mismatch
                    .push(format!("{}: read error", path.display()));
                continue;
            }
        };
        let plan: RalphPlan = match serde_json::from_str(&content) {
            Ok(p) => p,
            Err(e) => {
                report
                    .schema_mismatch
                    .push(format!("{}: parse error: {}", path.display(), e));
                continue;
            }
        };
        report.plans_read += 1;

        // Anti-lie validator (catches schema/version, accounting, verdict consistency).
        if let Err(e) = plan.validate() {
            // Classify by error variant: schema/accounting → fatal, others → soft_fail bucket.
            let msg = format!("{}: {}", path.display(), e);
            match e {
                dteam::ralph_plan::ValidationError::BadSchemaVersion { .. } => {
                    report.schema_mismatch.push(msg);
                }
                dteam::ralph_plan::ValidationError::AccountingUnbalanced { .. }
                | dteam::ralph_plan::ValidationError::PhaseSequenceLenMismatch { .. } => {
                    report.unbalanced.push(msg);
                }
                _ => {
                    report.false_completion.push(msg);
                }
            }
            continue;
        }

        // Phase-level pathologies that aren't structural lies but indicate operational pathologies.
        for completed in &plan.completed_phases {
            // Every completed phase must have at least one artifact whose `kind` matches.
            let has_artifact = plan.artifacts.iter().any(|a| &a.kind == completed);
            if !has_artifact {
                report.missing_artifact.push(format!(
                    "{}: phase '{}' marked completed without producing artifact",
                    path.display(),
                    completed
                ));
            }
        }
        for completed in &plan.completed_phases {
            // No phase may be completed if a gate referencing it failed.
            let failed = plan
                .gates
                .iter()
                .any(|g| g.status == GateStatus::Fail && g.name.starts_with(completed));
            if failed {
                report.false_completion.push(format!(
                    "{}: phase '{}' marked completed but gate failed",
                    path.display(),
                    completed
                ));
            }
        }
        if !plan.blocked_phases.is_empty() {
            report.blocked.push(format!(
                "{}: {} blocked phase(s)",
                path.display(),
                plan.blocked_phases.len()
            ));
        }
        if !plan.skipped_phases.is_empty() {
            report.skipped.push(format!(
                "{}: {} skipped phase(s) ({})",
                path.display(),
                plan.skipped_phases.len(),
                plan.skipped_phases.join(",")
            ));
        }
        match plan.verdict {
            Verdict::Pass => report.pass_count += 1,
            Verdict::SoftFail => report.soft_fail_count += 1,
            Verdict::Fatal => report.fatal_count += 1,
        }
    }

    // Top-level verdict synthesis.
    let any_fatal = !report.schema_mismatch.is_empty()
        || !report.unbalanced.is_empty()
        || !report.false_completion.is_empty()
        || report.fatal_count > 0;
    let any_soft = !report.missing_artifact.is_empty()
        || !report.blocked.is_empty()
        || !report.skipped.is_empty()
        || report.soft_fail_count > 0;
    let verdict = if any_fatal {
        "fatal"
    } else if any_soft {
        "soft_fail"
    } else {
        "pass"
    };

    if json_mode {
        let pathologies = build_ralph_pathologies(&report);
        let out = serde_json::json!({
            "kind": "ralph-plan",
            "plans_read": report.plans_read,
            "pathologies": pathologies,
            "counts": {
                "pass": report.pass_count,
                "soft_fail": report.soft_fail_count,
                "fatal": report.fatal_count,
            },
            "verdict": verdict,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        println!("ralph-plan doctor: {} plans, verdict={}", report.plans_read, verdict);
    }

    match verdict {
        "fatal" => ExitCode::from(2),
        "soft_fail" => ExitCode::from(1),
        _ => ExitCode::from(0),
    }
}

fn build_ralph_pathologies(report: &RalphPlanReport) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    if !report.schema_mismatch.is_empty() {
        out.push(serde_json::json!({
            "name": "SCHEMA_MISMATCH",
            "severity": "fatal",
            "count": report.schema_mismatch.len(),
            "message": format!("{} plan(s) failed schema validation", report.schema_mismatch.len()),
            "repair": "Fix RalphPlan schema version or regenerate plans.",
            "examples": report.schema_mismatch.iter().take(3).cloned().collect::<Vec<_>>()
        }));
    }
    if !report.unbalanced.is_empty() {
        out.push(serde_json::json!({
            "name": "UNBALANCED",
            "severity": "fatal",
            "count": report.unbalanced.len(),
            "message": format!("{} plan(s) have unbalanced phase accounting", report.unbalanced.len()),
            "repair": "completed + blocked + skipped + pending must equal phases_expected.",
            "examples": report.unbalanced.iter().take(3).cloned().collect::<Vec<_>>()
        }));
    }
    if !report.false_completion.is_empty() {
        out.push(serde_json::json!({
            "name": "FALSE_COMPLETION",
            "severity": "fatal",
            "count": report.false_completion.len(),
            "message": format!("{} false-completion violation(s)", report.false_completion.len()),
            "repair": "A phase cannot be marked completed if its gate failed; verdict must reflect gates.",
            "examples": report.false_completion.iter().take(3).cloned().collect::<Vec<_>>()
        }));
    }
    if !report.missing_artifact.is_empty() {
        out.push(serde_json::json!({
            "name": "MISSING_PRODUCING_ARTIFACT",
            "severity": "warn",
            "count": report.missing_artifact.len(),
            "message": format!("{} completed phase(s) without an artifact", report.missing_artifact.len()),
            "repair": "Every completed phase must record at least one artifact in `artifacts`.",
            "examples": report.missing_artifact.iter().take(3).cloned().collect::<Vec<_>>()
        }));
    }
    if !report.blocked.is_empty() {
        out.push(serde_json::json!({
            "name": "BLOCKED_PHASES",
            "severity": "warn",
            "count": report.blocked.len(),
            "message": format!("{} plan(s) report blocked phases", report.blocked.len()),
            "repair": "Resolve upstream gate failures or reclassify.",
            "examples": report.blocked.iter().take(3).cloned().collect::<Vec<_>>()
        }));
    }
    if !report.skipped.is_empty() {
        out.push(serde_json::json!({
            "name": "SKIPPED_PHASES",
            "severity": "warn",
            "count": report.skipped.len(),
            "message": format!("{} plan(s) report skipped phases", report.skipped.len()),
            "repair": "Either run the skipped phases or downgrade the canonical phase_sequence to match scope.",
            "examples": report.skipped.iter().take(3).cloned().collect::<Vec<_>>()
        }));
    }
    out
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let args = parse_args();

    // ── ralph-plan mode (distinct artifact family from AutomlPlan) ────────────
    if args.kind == DoctorKind::RalphPlan {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let default_plans_dir = working_dir.join("artifacts/ralph/ralph_plans");
        let plans_dir = args.plans_dir.as_deref().unwrap_or(&default_plans_dir);
        return run_ralph_plan_mode(plans_dir, args.json);
    }

    // ── Single-plan deep-dive mode ────────────────────────────────────────────
    if let Some(plan_path) = &args.plan {
        run_single_plan(plan_path);
        return ExitCode::from(0);
    }

    let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let default_plans_dir = working_dir.join("artifacts/pdc2025/automl_plans");
    let plans_dir = args.plans_dir.as_deref().unwrap_or(&default_plans_dir);
    let artifacts_dir = plans_dir.parent().unwrap_or(&working_dir);

    // ── JSON mode ─────────────────────────────────────────────────────────────
    if args.json {
        if !plans_dir.exists() {
            let out = serde_json::json!({
                "error": format!("plans directory not found: {}", plans_dir.display()),
                "verdict": "fatal"
            });
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
            return ExitCode::from(1);
        }
        let plans = load_plans(plans_dir);
        let truth = check_truth(&plans);
        let slow = check_slow(&plans);
        let saturation = check_saturation(&plans);
        let redundancy = check_redundancy(&plans);
        let tiers = tier_distribution(&plans);
        let head = get_git_head();
        let artifact_commit = get_artifact_commit(artifacts_dir);
        let git_match = match (&head, &artifact_commit) {
            (Some(h), Some(a)) => h == a,
            _ => false,
        };
        output_json(
            &plans,
            &truth,
            &slow,
            &saturation,
            &redundancy,
            &tiers,
            git_match,
            artifact_commit.as_deref(),
            head.as_deref(),
        );
        let lying = !truth.unbalanced.is_empty()
            || !truth.accounting_broken.is_empty()
            || !truth.pareto_invariant_broken.is_empty()
            || !truth.oracle_gap_lies.is_empty()
            || !truth.parse_errors.is_empty();
        return if lying {
            ExitCode::from(2)
        } else if !slow.warm_plans.is_empty() || !saturation.saturated_plans.is_empty() {
            ExitCode::from(1)
        } else {
            ExitCode::from(0)
        };
    }

    // ── Banner ────────────────────────────────────────────────────────────────
    println!(
        "{}{}┌─────────────────────────────────────────────────────────────────────────┐{}",
        BOLD, DIM, RESET
    );
    println!(
        "{}{}│ dteam doctor — epistemic diagnostic                                     │{}",
        BOLD, DIM, RESET
    );
    println!(
        "{}{}└─────────────────────────────────────────────────────────────────────────┘{}",
        BOLD, DIM, RESET
    );

    // Validate plans dir exists
    if !plans_dir.exists() {
        println!("  plans dir:   {}{}{}", DIM, plans_dir.display(), RESET);
        warn_line(
            "plans directory not found",
            &format!("{} — run `cargo make pdc` to generate", plans_dir.display()),
        );
        return ExitCode::from(1);
    }

    let plans = load_plans(plans_dir);
    let head = get_git_head();
    let artifact_commit = get_artifact_commit(artifacts_dir);

    println!("  plans dir:   {}{}{}", DIM, plans_dir.display(), RESET);
    println!("  plans found: {}{}{}", DIM, plans.len(), RESET);
    println!(
        "  git:         {}{}{}",
        DIM,
        head.as_deref().unwrap_or("(unknown)"),
        RESET
    );

    if plans.is_empty() {
        warn_line(
            "no plan JSON files found",
            "run `cargo make pdc` with [automl].enabled=true",
        );
        return ExitCode::from(1);
    }

    // ── --target mode ─────────────────────────────────────────────────────────
    if let Some(target_str) = &args.target {
        match target_tier_from_str(target_str) {
            Some(tier) => {
                run_target_mode(&tier, &plans);
                return ExitCode::from(0);
            }
            None => {
                println!(
                    "  {}unknown target '{}' — valid: T0 T1 T2 warm{}",
                    RED, target_str, RESET
                );
                return ExitCode::from(1);
            }
        }
    }

    // ── Full diagnostic ───────────────────────────────────────────────────────

    let truth = check_truth(&plans);
    let slow = check_slow(&plans);
    let saturation = check_saturation(&plans);
    let redundancy = check_redundancy(&plans);
    let tiers = tier_distribution(&plans);

    let mut exit_code: u8 = 0;
    let mut pathology_names: Vec<String> = vec![];

    // ── Truth ─────────────────────────────────────────────────────────────────
    section("Truth");

    let parse_ok = truth.parse_errors.is_empty();
    check(
        "plan JSON parseable",
        parse_ok,
        &if parse_ok {
            format!("{}/{} plans", plans.len(), plans.len())
        } else {
            format!("{} parse errors", truth.parse_errors.len())
        },
    );
    for e in &truth.parse_errors {
        println!("    {}• {}{}", RED, e, RESET);
    }

    let balanced_ok = truth.unbalanced.is_empty();
    check(
        "accounting_balanced",
        balanced_ok,
        &if balanced_ok {
            format!("{}/{} plans balanced", plans.len(), plans.len())
        } else {
            format!("{} unbalanced plans", truth.unbalanced.len())
        },
    );
    for name in &truth.unbalanced {
        println!("    {}• {}{}", RED, name, RESET);
    }

    let acct_ok = truth.accounting_broken.is_empty();
    check(
        "signal count identity",
        acct_ok,
        &if acct_ok {
            "selected + rej_corr + rej_gain = evaluated in all plans".to_string()
        } else {
            format!("{} broken", truth.accounting_broken.len())
        },
    );
    for e in &truth.accounting_broken {
        println!("    {}• {}{}", RED, e, RESET);
    }

    let pareto_ok = truth.pareto_invariant_broken.is_empty();
    check(
        "pareto chosen invariant",
        pareto_ok,
        &if pareto_ok {
            "exactly 1 chosen per plan".to_string()
        } else {
            format!("{} violations", truth.pareto_invariant_broken.len())
        },
    );
    for e in &truth.pareto_invariant_broken {
        println!("    {}• {}{}", RED, e, RESET);
    }

    let oracle_ok = truth.oracle_gap_lies.is_empty();
    check(
        "oracle gap formula",
        oracle_ok,
        &if oracle_ok {
            "all within 1e-6 tolerance".to_string()
        } else {
            format!("{} lies detected", truth.oracle_gap_lies.len())
        },
    );
    for e in &truth.oracle_gap_lies {
        println!("    {}• {}{}", RED, e, RESET);
    }

    let lying = !parse_ok || !balanced_ok || !acct_ok || !pareto_ok || !oracle_ok;
    if lying {
        exit_code = 2;
        let lie_count = truth.parse_errors.len()
            + truth.unbalanced.len()
            + truth.accounting_broken.len()
            + truth.pareto_invariant_broken.len()
            + truth.oracle_gap_lies.len();
        pathology_names.push(format!(
            "LYING ({} violation{})",
            lie_count,
            if lie_count == 1 { "" } else { "s" }
        ));
    }

    // ── Speed ─────────────────────────────────────────────────────────────────
    section("Speed");

    // Informational tier counts (always pass=true — just reporting)
    check(
        &format!("T0 plans (<{})", fmt_us(T0_MAX_US)),
        true,
        &tiers.t0.to_string(),
    );
    check(
        &format!("T1 plans (<{})", fmt_us(T1_MAX_US)),
        true,
        &tiers.t1.to_string(),
    );
    check(
        &format!("T2 plans (<{})", fmt_us(T2_MAX_US)),
        true,
        &tiers.t2.to_string(),
    );

    if slow.warm_plans.is_empty() {
        check("Warm plans (>100ms)", true, "0  → all plans within T2");
    } else {
        warn_line(
            "Warm plans (>100ms)",
            &format!("{}  → SLOW", slow.warm_plans.len()),
        );
        if !slow.warm_signals.is_empty() {
            let sig_str: Vec<String> = slow
                .warm_signals
                .iter()
                .take(5)
                .map(|(s, us)| format!("{} ({})", s, fmt_us(*us)))
                .collect();
            println!("    {}signals: {}{}", DIM, sig_str.join(", "), RESET);
        }
        if let Some(dg) = &slow.cheapest_t2_downgrade {
            println!(
                "    {}cheapest T2 downgrade: {} ({}, Δacc {:.1}%){}",
                DIM,
                dg.signals.join("+"),
                fmt_us(dg.timing_us),
                dg.acc_delta * 100.0,
                RESET
            );
        }
        if exit_code < 1 {
            exit_code = 1;
        }
        pathology_names.push(format!("SLOW ({} plans)", slow.warm_plans.len()));
    }

    // ── Diversity ─────────────────────────────────────────────────────────────
    section("Diversity");

    if saturation.saturated_plans.is_empty() {
        check(
            "signal family diversity",
            true,
            "all plans use mixed signal families",
        );
    } else {
        warn_line(
            "conformance monoculture",
            &format!(
                "{}/{} plans  → SATURATED",
                saturation.saturated_plans.len(),
                plans.len()
            ),
        );
        if let Some(fam) = &saturation.dominant_family {
            let advice = match fam {
                SignalFamily::Conformance => {
                    "add TF-IDF (T2) or HDC (T2) for orthogonal projection signal"
                }
                SignalFamily::NlpSymbolic => {
                    "add conformance signal (H/G/F) for process-structure coverage"
                }
                SignalFamily::HyperdimSynthetic => {
                    "add TF-IDF (T2) or conformance signal for lexical coverage"
                }
                SignalFamily::AutomlRl => {
                    "add conformance signal (H/G/F) for deterministic baseline"
                }
                SignalFamily::MlEnsemble => {
                    "add conformance signal (H/G/F) for process-structure coverage"
                }
            };
            println!(
                "    {}dominant family: {}  — {}{}",
                DIM,
                fam.name(),
                advice,
                RESET
            );
        }
        if exit_code < 1 {
            exit_code = 1;
        }
        pathology_names.push(format!(
            "SATURATED ({} plans)",
            saturation.saturated_plans.len()
        ));
    }

    if redundancy.high_redundancy_plans.is_empty() {
        let corr_pct = if tiers.total > 0 {
            let total_corr: usize = plans
                .iter()
                .filter(|p| p.parse_error.is_none())
                .map(|p| p.signals_rejected_correlation)
                .sum();
            let total_eval: usize = plans
                .iter()
                .filter(|p| p.parse_error.is_none())
                .map(|p| p.signals_evaluated)
                .sum();
            if total_eval > 0 {
                total_corr as f64 / total_eval as f64 * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        info_line(
            "rejected for correlation",
            &format!(
                "{:.1} avg/plan ({:.1}% of evaluated — healthy)",
                redundancy.avg_correlation_rejected, corr_pct
            ),
        );
    } else {
        warn_line(
            "rejected for correlation",
            &format!(
                "{:.1} avg/plan (>40% in {} plans — diversify signal pool)",
                redundancy.avg_correlation_rejected,
                redundancy.high_redundancy_plans.len()
            ),
        );
        pathology_names.push(format!(
            "REDUNDANT ({} plans)",
            redundancy.high_redundancy_plans.len()
        ));
    }

    // ── Deployability ──────────────────────────────────────────────────────────
    section("Deployability");

    let browser = tiers.t0;
    let edge = tiers.t0 + tiers.t1;
    let fog = tiers.t0 + tiers.t1 + tiers.t2;
    let cloud = tiers.total;

    println!(
        "  {}  browser/WASM  T0 <{}    {}/{:<5}  {}{}{}",
        DIM,
        fmt_us(T0_MAX_US),
        browser,
        tiers.total,
        if browser > 0 { GREEN } else { YELLOW },
        if browser > 0 { "✓" } else { "✗" },
        RESET
    );
    println!(
        "  {}  edge/CDN      T1 <{}   {}/{:<5}  {}{}{}",
        DIM,
        fmt_us(T1_MAX_US),
        edge,
        tiers.total,
        if edge > 0 { GREEN } else { YELLOW },
        if edge > 0 { "✓" } else { "✗" },
        RESET
    );
    println!(
        "  {}  fog/svcless   T2 <{}  {}/{:<5}  {}{}{}",
        DIM,
        fmt_us(T2_MAX_US),
        fog,
        tiers.total,
        if fog > 0 { GREEN } else { YELLOW },
        if fog > 0 { "✓" } else { "✗" },
        RESET
    );
    println!(
        "  {}  cloud         Warm <∞   {}/{:<5}  {}✓{}",
        DIM, cloud, tiers.total, GREEN, RESET
    );

    // ── Staleness ─────────────────────────────────────────────────────────────
    section("Staleness");

    match (&head, &artifact_commit) {
        (Some(h), Some(a)) => {
            if h == a {
                check("artifact commit", true, &format!("{} matches HEAD", a));
            } else {
                warn_line(
                    "artifact commit",
                    &format!("{} ≠ HEAD {} — STALE, run `cargo make pdc`", a, h),
                );
                if exit_code < 1 {
                    exit_code = 1;
                }
                pathology_names.push("STALE".to_string());
            }
        }
        (Some(h), None) => {
            warn_line(
                "artifact commit",
                &format!("run_metadata.json missing — HEAD is {} — STALE", h),
            );
            if exit_code < 1 {
                exit_code = 1;
            }
            pathology_names.push("STALE".to_string());
        }
        (None, Some(a)) => {
            info_line(
                "artifact commit",
                &format!("{} (git unavailable for HEAD comparison)", a),
            );
        }
        (None, None) => {
            info_line(
                "artifact commit",
                "run_metadata.json missing and git unavailable",
            );
        }
    }

    // ── Verdict ───────────────────────────────────────────────────────────────
    section("Verdict");

    if exit_code == 0 {
        println!("  {}{}✓ HEALTHY — all checks pass{}", BOLD, GREEN, RESET);
        println!(
            "  {}T0:{} T1:{} T2:{} Warm:{} — {} plans{}",
            DIM,
            tiers.t0,
            tiers.t1,
            tiers.t2,
            tiers.warm,
            plans.len(),
            RESET
        );
    } else if exit_code == 2 {
        println!("  {}{}✗ FATAL — LYING detected{}", BOLD, RED, RESET);
        println!(
            "  {}invariant violated — pipeline artifacts cannot be trusted{}",
            DIM, RESET
        );
        println!(
            "  {}pathologies: {}{}",
            DIM,
            pathology_names.join(" · "),
            RESET
        );
    } else {
        println!(
            "  {}{}⚠ SOFT FAIL — {} patholog{}: {}{}",
            BOLD,
            YELLOW,
            pathology_names.len(),
            if pathology_names.len() == 1 {
                "y"
            } else {
                "ies"
            },
            pathology_names.join(" · "),
            RESET
        );
        println!(
            "  {}run `cargo make pdc` to regenerate with corrected configuration{}",
            DIM, RESET
        );
    }

    println!();
    ExitCode::from(exit_code)
}
