//! `dteam conformance` CLI.
//!
//! Replays an OCEL JSONL event stream against a declared PNML Petri net,
//! grouping events into cases by a chosen object-type id, and emits a JSON
//! report with fitness / precision / generalization / simplicity metrics.
//!
//! Exit codes:
//!   0 — all enabled metrics meet thresholds
//!   2 — at least one metric below threshold
//!   1 — runtime error (parse / IO)

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::Value;

use dteam::conformance::token_replay;
use dteam::io::pnml::read_pnml;
use dteam::models::petri_net::PetriNet;
use dteam::models::{Attribute, AttributeValue, Event, EventLog, Trace};

#[derive(Default)]
struct Args {
    log: Option<PathBuf>,
    model: Option<PathBuf>,
    object_type: Option<String>,
    metrics: Vec<String>,
    threshold_fitness: f64,
    threshold_precision: f64,
    threshold_generalization: f64,
    threshold_simplicity: f64,
    report: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut args = Args {
        metrics: vec![
            "fitness".into(),
            "precision".into(),
            "generalization".into(),
            "simplicity".into(),
        ],
        threshold_fitness: 0.99,
        threshold_precision: 0.0,
        threshold_generalization: 0.0,
        threshold_simplicity: 0.0,
        ..Default::default()
    };

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--log" => args.log = Some(it.next().context("--log needs value")?.into()),
            "--model" => args.model = Some(it.next().context("--model needs value")?.into()),
            "--object-type" => {
                args.object_type = Some(it.next().context("--object-type needs value")?)
            }
            "--metrics" => {
                let v = it.next().context("--metrics needs value")?;
                args.metrics = v.split(',').map(|s| s.trim().to_string()).collect();
            }
            "--threshold-fitness" => args.threshold_fitness = it.next().context("v")?.parse()?,
            "--threshold-precision" => {
                args.threshold_precision = it.next().context("v")?.parse()?
            }
            "--threshold-generalization" => {
                args.threshold_generalization = it.next().context("v")?.parse()?
            }
            "--threshold-simplicity" => {
                args.threshold_simplicity = it.next().context("v")?.parse()?
            }
            "--report" => args.report = Some(it.next().context("--report needs value")?.into()),
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(anyhow!("unknown arg: {}", other)),
        }
    }

    if args.log.is_none() || args.model.is_none() || args.object_type.is_none() {
        return Err(anyhow!("--log, --model, --object-type are required"));
    }
    Ok(args)
}

fn print_help() {
    println!(
        "dteam conformance --log <ocel.jsonl> --model <net.pnml> --object-type <Type> \\\n  [--metrics fitness,precision,generalization,simplicity] \\\n  [--threshold-fitness 0.99] [--threshold-precision 0.0] \\\n  [--threshold-generalization 0.0] [--threshold-simplicity 0.0] \\\n  [--report <out.json>]"
    );
}

#[derive(Serialize)]
struct MetricResult {
    value: f64,
    threshold: f64,
    pass: bool,
}

#[derive(Serialize)]
struct Report {
    log: String,
    model: String,
    object_type: String,
    cases: usize,
    events: usize,
    activities: Vec<String>,
    transitions_in_model: usize,
    metrics: BTreeMap<String, MetricResult>,
    verdict: String,
}

fn build_event_log(path: &PathBuf, object_type: &str) -> Result<(EventLog, usize)> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    // case_id (object id) -> Vec<(timestamp, activity)>
    let mut cases: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut total_events = 0usize;
    for (lineno, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: Value =
            serde_json::from_str(line).with_context(|| format!("parse line {}", lineno + 1))?;
        let activity = v
            .get("ocel:activity")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let ts = v
            .get("ocel:timestamp")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        if activity.is_empty() {
            continue;
        }
        let objs = match v.get("ocel:objects").and_then(|o| o.as_array()) {
            Some(a) => a,
            None => continue,
        };
        let mut matched = false;
        for o in objs {
            let ty = o.get("type").and_then(|x| x.as_str()).unwrap_or("");
            if ty == object_type {
                if let Some(id) = o.get("id").and_then(|x| x.as_str()) {
                    cases
                        .entry(id.to_string())
                        .or_default()
                        .push((ts.clone(), activity.to_string()));
                    matched = true;
                }
            }
        }
        if matched {
            total_events += 1;
        }
    }

    let mut log = EventLog::default();
    for (case_id, mut evs) in cases {
        evs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut trace = Trace::new(case_id);
        for (_, act) in evs {
            trace.events.push(Event {
                attributes: vec![Attribute {
                    key: "concept:name".into(),
                    value: AttributeValue::String(act),
                }],
            });
        }
        log.traces.push(trace);
    }
    Ok((log, total_events))
}

/// Fraction of model transition labels that actually appear in the log.
/// Cheap proxy for ETC-precision: a model that allows behaviour never seen
/// is over-permissive, dropping precision.
fn precision_proxy(log: &EventLog, net: &PetriNet) -> f64 {
    let mut log_acts = std::collections::HashSet::new();
    for t in &log.traces {
        for e in &t.events {
            for a in &e.attributes {
                if a.key == "concept:name" {
                    if let AttributeValue::String(s) = &a.value {
                        log_acts.insert(s.clone());
                    }
                }
            }
        }
    }
    let visible_trans: Vec<&str> = net
        .transitions
        .iter()
        .map(|t| t.label.as_str())
        .filter(|l| !l.is_empty())
        .collect();
    if visible_trans.is_empty() {
        return 1.0;
    }
    let used = visible_trans
        .iter()
        .filter(|l| log_acts.contains(**l))
        .count();
    used as f64 / visible_trans.len() as f64
}

/// Van der Aalst generalization heuristic: 1 - 1/sqrt(N+1)
/// where N is the number of (activity-execution) observations.
fn generalization_proxy(log: &EventLog) -> f64 {
    let n: usize = log.traces.iter().map(|t| t.events.len()).sum();
    1.0 - 1.0 / ((n as f64 + 1.0).sqrt())
}

/// Simplicity: inverse of average node degree excess.
/// Fewer arcs per node => simpler net.
fn simplicity_proxy(net: &PetriNet) -> f64 {
    let nodes = net.places.len() + net.transitions.len();
    if nodes == 0 {
        return 1.0;
    }
    let avg_degree = net.arcs.len() as f64 / nodes as f64;
    1.0 / (1.0 + (avg_degree - 1.0).max(0.0))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {:#}", e);
            return ExitCode::from(1);
        }
    };

    let log_path = args.log.as_ref().unwrap();
    let model_path = args.model.as_ref().unwrap();
    let object_type = args.object_type.as_ref().unwrap();

    let net = match read_pnml(model_path.as_path()) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("error reading PNML: {:#}", e);
            return ExitCode::from(1);
        }
    };

    let (log, total_events) = match build_event_log(log_path, object_type) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error reading OCEL: {:#}", e);
            return ExitCode::from(1);
        }
    };

    let activities: Vec<String> = {
        let mut s = std::collections::BTreeSet::new();
        for t in &log.traces {
            for e in &t.events {
                for a in &e.attributes {
                    if a.key == "concept:name" {
                        if let AttributeValue::String(v) = &a.value {
                            s.insert(v.clone());
                        }
                    }
                }
            }
        }
        s.into_iter().collect()
    };

    let mut metrics: BTreeMap<String, MetricResult> = BTreeMap::new();
    let mut all_pass = true;
    let want = |m: &str| args.metrics.iter().any(|x| x.eq_ignore_ascii_case(m));

    if want("fitness") {
        let results = token_replay(&log, &net);
        let fitness = if results.is_empty() {
            1.0
        } else {
            results.iter().map(|r| r.fitness).sum::<f64>() / results.len() as f64
        };
        let pass = fitness >= args.threshold_fitness;
        all_pass &= pass;
        metrics.insert(
            "fitness".into(),
            MetricResult {
                value: fitness,
                threshold: args.threshold_fitness,
                pass,
            },
        );
    }
    if want("precision") {
        let v = precision_proxy(&log, &net);
        let pass = v >= args.threshold_precision;
        all_pass &= pass;
        metrics.insert(
            "precision".into(),
            MetricResult {
                value: v,
                threshold: args.threshold_precision,
                pass,
            },
        );
    }
    if want("generalization") {
        let v = generalization_proxy(&log);
        let pass = v >= args.threshold_generalization;
        all_pass &= pass;
        metrics.insert(
            "generalization".into(),
            MetricResult {
                value: v,
                threshold: args.threshold_generalization,
                pass,
            },
        );
    }
    if want("simplicity") {
        let v = simplicity_proxy(&net);
        let pass = v >= args.threshold_simplicity;
        all_pass &= pass;
        metrics.insert(
            "simplicity".into(),
            MetricResult {
                value: v,
                threshold: args.threshold_simplicity,
                pass,
            },
        );
    }

    let report = Report {
        log: log_path.display().to_string(),
        model: model_path.display().to_string(),
        object_type: object_type.clone(),
        cases: log.traces.len(),
        events: total_events,
        activities,
        transitions_in_model: net.transitions.len(),
        metrics,
        verdict: if all_pass {
            "pass".into()
        } else {
            "fail".into()
        },
    };

    let serialized = serde_json::to_string_pretty(&report).unwrap();
    if let Some(p) = &args.report {
        if let Err(e) = std::fs::write(p, &serialized) {
            eprintln!("error writing report: {}", e);
            return ExitCode::from(1);
        }
    } else {
        println!("{}", serialized);
    }

    if all_pass {
        ExitCode::from(0)
    } else {
        ExitCode::from(2)
    }
}
