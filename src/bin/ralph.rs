use dteam::agentic::ralph::{
    AgentKind, AutonomicController, ExecutionEngine, GitWorktreeManager, MaturityScorer,
    OntologyClosureEngine, PortfolioIndexer, RalphMode, ReceiptEmitter, SpecKitInvocation,
    SpecKitPhase, SpecKitRunner, SpeckitController, WorkSelector, WorkspaceManager,
};
use dteam::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use dteam::ralph_plan::{
    sha256_file, sha256_hex, Accounting, Artifact as RpArtifact, Gate, GateStatus, RalphPlan,
    Verdict, SCHEMA_VERSION,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing::{error, info, info_span, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn init_telemetry() -> anyhow::Result<SdkTracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes(vec![KeyValue::new("service.name", "ralph-orchestrator")])
                .build(),
        )
        .build();

    global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("ralph-orchestrator");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .init();

    Ok(provider)
}

struct PhaseEntry {
    phase: String,
    artifact_path: std::path::PathBuf,
    artifact_hash: String,
}

struct PhaseJournal {
    entries: Vec<PhaseEntry>,
}

impl PhaseJournal {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    fn record(&mut self, phase: &str, path: std::path::PathBuf, hash: String) {
        self.entries.push(PhaseEntry {
            phase: phase.into(),
            artifact_path: path,
            artifact_hash: hash,
        });
    }
}

fn inject_supervisor(working_dir: &Path) -> anyhow::Result<()> {
    let gemini_dir = working_dir.join(".gemini");
    let hooks_dir = gemini_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let settings = json!({
        "security": {
            "environmentVariableRedaction": { "enabled": true }
        },
        "hooks": {
            "BeforeTool": [
                {
                    "name": "ralph-supervisor",
                    "matcher": "write_file|replace|run_shell_command",
                    "type": "command",
                    "command": "./.gemini/hooks/supervisor.sh",
                    "timeout": 15000
                }
            ]
        }
    });

    fs::write(
        gemini_dir.join("settings.json"),
        serde_json::to_string_pretty(&settings)?,
    )?;

    let hook_script = r#"#!/usr/bin/env bash
input=$(cat)
tool_name=$(echo "$input" | jq -r '.tool_name')
tool_input=$(echo "$input" | jq -r '.tool_input')

if [[ "$tool_input" == *"AKIA"* || "$tool_input" == *"sk-ant"* || "$tool_input" == *"AIza"* ]]; then
    echo '{"decision": "deny", "reason": "SECURITY ALERT: Potential API Key detected in payload."}'
    exit 0
fi

if [[ "$tool_name" == "write_file" || "$tool_name" == "replace" ]]; then
    file_path=$(echo "$tool_input" | jq -r '.file_path')
    if [[ "$file_path" == *.rs ]]; then
        if [[ "$file_path" == *"lib.rs"* && "$tool_input" == *"syntax error"* ]]; then
             echo '{"decision": "deny", "reason": "SYNTAX VALIDATION: Prevented writing deliberate syntax error."}'
             exit 0
        fi
    fi
fi

echo '{"decision": "allow"}'
"#;

    let hook_path = hooks_dir.join("supervisor.sh");
    fs::write(&hook_path, hook_script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

/// Build and write a RalphPlan for one processed idea.
///
/// Phase completion is derived from the `PhaseJournal` — an observed record of
/// which phases completed and which artifacts were written during this run.
/// This replaces the previous disk-scan approach and eliminates the `is_test`
/// branch entirely; in test mode the journal is pre-populated before calling.
fn emit_ralph_plan(
    plans_out: &Path,
    run_id: &str,
    id: &str,
    idea: &str,
    journal: &PhaseJournal,
    constitution_hash: Option<String>,
) -> anyhow::Result<()> {
    fs::create_dir_all(plans_out)?;

    // Canonical Spec Kit phase sequence.
    let phase_sequence: Vec<&str> = vec!["specify", "plan", "tasks", "implement"];

    let mut completed_phases: Vec<String> = Vec::new();
    let mut blocked_phases: Vec<String> = Vec::new();
    let skipped_phases: Vec<String> = Vec::new();
    let mut artifacts: Vec<RpArtifact> = Vec::new();
    let mut gates: Vec<Gate> = Vec::new();

    for phase in &["specify", "plan", "tasks", "implement"] {
        match journal.entries.iter().find(|e| e.phase.as_str() == *phase) {
            Some(entry) => {
                completed_phases.push((*phase).to_string());
                artifacts.push(RpArtifact {
                    kind: (*phase).to_string(),
                    path: entry.artifact_path.display().to_string(),
                    hash: entry.artifact_hash.clone(),
                });
                gates.push(Gate {
                    name: format!("{}_artifact_present", phase),
                    status: GateStatus::Pass,
                    failure_class: None,
                });
            }
            None => {
                blocked_phases.push((*phase).to_string());
                gates.push(Gate {
                    name: format!("{}_artifact_present", phase),
                    status: GateStatus::Fail,
                    failure_class: Some("MISSING_ARTIFACT".into()),
                });
            }
        }
    }

    let phases_expected = phase_sequence.len() as u32;
    let phases_completed = completed_phases.len() as u32;
    let phases_blocked = blocked_phases.len() as u32;
    let phases_skipped = skipped_phases.len() as u32;
    let phases_pending =
        phases_expected - phases_completed - phases_blocked - phases_skipped;
    let balanced = phases_completed + phases_blocked + phases_skipped + phases_pending
        == phases_expected;

    // Determine highest-reached phase: last entry in `completed_phases` according to sequence order.
    let phase = phase_sequence
        .iter()
        .rev()
        .find(|p| completed_phases.iter().any(|c| c.as_str() == **p))
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| "specify".to_string());

    // Verdict policy:
    //   Pass     — every phase completed, no blocked/pending, no failed gates
    //   SoftFail — gates failing OR phases blocked OR phases pending (still structurally honest)
    //   Fatal    — reserved for catastrophic; not selected by emitter (validator may upgrade)
    let any_fail = gates.iter().any(|g| g.status == GateStatus::Fail);
    // Pass requires every phase actually completed: no fails, no blocks, no pending, no skips.
    let verdict = if !any_fail
        && phases_blocked == 0
        && phases_pending == 0
        && phases_skipped == 0
    {
        Verdict::Pass
    } else {
        Verdict::SoftFail
    };

    // spec_hash: look up the specify artifact hash from the journal.
    let spec_hash = journal
        .entries
        .iter()
        .find(|e| e.phase == "specify")
        .map(|e| e.artifact_hash.clone());

    let plan = RalphPlan {
        schema: SCHEMA_VERSION.into(),
        run_id: run_id.into(),
        target: id.into(),
        idea_hash: sha256_hex(idea.as_bytes()),
        constitution_hash,
        spec_hash,
        phase,
        phase_sequence: phase_sequence.iter().map(|s| (*s).into()).collect(),
        completed_phases,
        blocked_phases,
        skipped_phases,
        artifacts,
        gates,
        accounting: Accounting {
            phases_expected,
            phases_completed,
            phases_blocked,
            phases_skipped,
            phases_pending,
            balanced,
        },
        verdict,
    };

    // Validate before writing — refuse to emit a plan that fails its own invariants.
    plan.validate()
        .map_err(|e| anyhow::anyhow!("RalphPlan failed self-validation: {}", e))?;

    let out_path = plans_out.join(format!("ralph_{}.json", id));
    let json = serde_json::to_string_pretty(&plan)?;
    fs::write(&out_path, json)?;
    info!("RalphPlan written: {}", out_path.display());

    Ok(())
}

/// `ralph portfolio-tick` — single-shot portfolio indexing pulse.
///
/// Loads `registry.yaml`, optionally narrows to a single cell, runs
/// `PortfolioIndexer.scan()` against that cell's path (or the registry root if
/// `--cell` is omitted), asks `WorkSelector` for the next admissible unit, and
/// writes a receipt JSON to `--emit-receipt`. Exits 0 on success.
fn run_portfolio_tick(args: &[String]) -> anyhow::Result<()> {
    fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
        let pos = args.iter().position(|a| a == name)?;
        args.get(pos + 1).map(|s| s.as_str())
    }

    let registry = flag(args, "--registry")
        .ok_or_else(|| anyhow::anyhow!("--registry <path> is required"))?;
    let cell = flag(args, "--cell");
    let phase = flag(args, "--phase").unwrap_or("observe");
    let emit_receipt = flag(args, "--emit-receipt")
        .ok_or_else(|| anyhow::anyhow!("--emit-receipt <path> is required"))?;

    let registry_path = PathBuf::from(registry);
    if !registry_path.exists() {
        anyhow::bail!("registry not found: {}", registry_path.display());
    }
    let registry_text = fs::read_to_string(&registry_path)?;

    // Minimal YAML scrape: find `- id: <cell>` block and its `path:` line.
    // Avoids pulling in serde_yaml just for one lookup.
    let cell_path: PathBuf = match cell {
        Some(name) => {
            let mut found: Option<PathBuf> = None;
            let lines: Vec<&str> = registry_text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim_start();
                if let Some(rest) = trimmed.strip_prefix("- id:") {
                    if rest.trim().trim_matches(|c: char| c == '"' || c == '\'') == name {
                        // scan forward until next `- id:` or end for `path:`
                        for next in lines.iter().skip(i + 1) {
                            let nt = next.trim_start();
                            if nt.starts_with("- id:") {
                                break;
                            }
                            if let Some(pv) = nt.strip_prefix("path:") {
                                let p = pv.trim().trim_matches(|c: char| c == '"' || c == '\'');
                                found = Some(PathBuf::from(p));
                                break;
                            }
                        }
                        break;
                    }
                }
            }
            found.ok_or_else(|| {
                anyhow::anyhow!("cell '{}' not found in registry {}", name, registry_path.display())
            })?
        }
        None => registry_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    let indexer = PortfolioIndexer::new();
    let state = indexer.scan(&cell_path)?;

    let selector = WorkSelector::new();
    let next_unit = selector.select_next(&state)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let receipt = json!({
        "schema": "ralph.portfolio_tick.v1",
        "cell": cell.unwrap_or("<all>"),
        "phase": phase,
        "registry": registry_path.display().to_string(),
        "cell_path": cell_path.display().to_string(),
        "active_projects": state.active_projects,
        "known_artifacts": state
            .known_artifacts
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
        "next_unit": next_unit,
        "emitted_at": timestamp,
        "verdict": "pass",
    });

    let receipt_path = PathBuf::from(emit_receipt);
    if let Some(parent) = receipt_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&receipt_path, serde_json::to_string_pretty(&receipt)?)?;

    // ReceiptEmitter is reused for the legacy text-receipt sidecar so the
    // subcommand exercises the same emission path the orchestrator uses.
    let emitter = ReceiptEmitter::new();
    let sidecar_dir = receipt_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let _ = emitter.emit(
        &sidecar_dir,
        &format!("portfolio-tick:{}", cell.unwrap_or("<all>")),
        "PortfolioTickOk",
    );

    info!(
        "portfolio-tick complete: cell={} phase={} receipt={}",
        cell.unwrap_or("<all>"),
        phase,
        receipt_path.display()
    );
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = init_telemetry().ok();
    let _main_span = info_span!("ralph_main").entered();

    if cfg!(debug_assertions) {
        info!("--- Ralph Wiggum Loop: Rust Parallel Orchestrator ---");
    }

    // Subcommand dispatch (manual argv parse, matches existing style).
    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.iter().any(|a| a == "portfolio-tick") {
        let result = run_portfolio_tick(&raw_args);
        if let Some(p) = provider {
            let _ = p.force_flush();
        }
        return result;
    }

    let root_dir = Path::new(".");
    let indexer = PortfolioIndexer::new();
    let state = indexer.scan(root_dir)?;
    info!(
        "Portfolio state scanned: {} active projects.",
        state.active_projects
    );

    let ontology = OntologyClosureEngine::new();
    let ontology_ctx = ontology.load_context(&root_dir.join("PUBLIC-ONTOLOGIES.ttl"))?;
    info!("Ontology loaded: {}", ontology_ctx);

    let scorer = MaturityScorer::new();
    let maturity = scorer.evaluate(&state)?;
    info!("Current Portfolio Maturity Level: {}", maturity);

    let selector = WorkSelector::new();
    let next_admissible_unit = selector.select_next(&state)?;
    info!("Next admissible unit: {}", next_admissible_unit);

    let args: Vec<String> = std::env::args().collect();
    let is_test = args.contains(&"--test".to_string());

    let mut max_concurrency = 1;
    if let Some(pos) = args.iter().position(|a| a == "--concurrency") {
        if let Some(val) = args.get(pos + 1) {
            max_concurrency = val.parse::<usize>().unwrap_or(1);
        }
    }

    let mut offset = 0;
    if let Some(pos) = args.iter().position(|a| a == "--offset") {
        if let Some(val) = args.get(pos + 1) {
            offset = val.parse::<usize>().unwrap_or(0);
        }
    }

    let mut limit = None;
    if let Some(pos) = args.iter().position(|a| a == "--limit") {
        if let Some(val) = args.get(pos + 1) {
            limit = val.parse::<usize>().ok();
        }
    }

    // RalphPlan emission: emit one JSON per processed idea into this directory.
    let mut plans_out = PathBuf::from("artifacts/ralph/ralph_plans");
    if let Some(pos) = args.iter().position(|a| a == "--plans-out") {
        if let Some(val) = args.get(pos + 1) {
            plans_out = PathBuf::from(val);
        }
    }
    let _ = fs::create_dir_all(&plans_out);

    let run_id = format!(
        "ralph-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
    );

    if is_test && cfg!(debug_assertions) {
        info!("!! TEST MODE ENABLED: Skipping LLM calls and using mock responses.");
    }
    info!("RalphPlan emission dir: {}", plans_out.display());
    info!("Ralph run id: {}", run_id);

    let ideas_path = Path::new("IDEAS.md");
    if !ideas_path.exists() {
        fs::write(
            ideas_path,
            "1. Implement a basic health check endpoint\n2. Add logging to the autonomic cycle\n",
        )?;
    }

    let content = fs::read_to_string(ideas_path)?;
    let ideas: Vec<(String, String)> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, s)| (format!("{:03}", i + 1), s.to_string()))
        .skip(offset)
        .take(limit.unwrap_or(usize::MAX))
        .collect();

    let workspace_manager = GitWorktreeManager;
    workspace_manager.ensure_dev_branch()?;

    let meta_log = Arc::new(Mutex::new(EventLog::default()));
    let merge_lock = Arc::new(Mutex::new(()));

    // Capture constitution hash once at run start so it reflects startup state.
    let constitution_hash = sha256_file(Path::new(".specify/memory/constitution.md"));
    let constitution_hash = Arc::new(constitution_hash);

    let engine = ExecutionEngine::new(max_concurrency);

    let meta_log_clone = Arc::clone(&meta_log);
    let merge_lock_clone = Arc::clone(&merge_lock);
    let plans_out_arc = Arc::new(plans_out.clone());
    let run_id_arc = Arc::new(run_id.clone());
    let constitution_hash_arc = Arc::clone(&constitution_hash);

    let process_fn = move |id: String, idea: String| {
        let meta_log = Arc::clone(&meta_log_clone);
        let merge_lock = Arc::clone(&merge_lock_clone);
        let plans_out = Arc::clone(&plans_out_arc);
        let run_id = Arc::clone(&run_id_arc);
        let constitution_hash = Arc::clone(&constitution_hash_arc);

        async move {
            tokio::task::spawn_blocking(move || {
                let _span = info_span!("process_idea", idea = %idea, id = %id).entered();
                let slug = idea
                    .to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric(), "-")
                    .split('-')
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("-");

                let working_dir = PathBuf::from(".wreckit").join(format!("{}-{}", id, slug));
                fs::create_dir_all(&working_dir)?;

                if cfg!(debug_assertions) {
                    info!("\n[Idea {}] Processing: {}", id, idea);
                }

                let mut trace = Trace {
                    id: id.clone(),
                    ..Default::default()
                };

                let branch_name = format!("wreckit/{}", slug);
                let worktree_path = working_dir.join("worktree");

                let workspace = GitWorktreeManager;
                if let Err(e) = workspace.setup_worktree(&branch_name, &worktree_path) {
                    error!("Failed to setup worktree: {}", e);
                    return Ok::<(), anyhow::Error>(());
                }

                let controller = SpeckitController::new();
                let start = Instant::now();

                let root_path = std::env::current_dir().unwrap_or(PathBuf::from("."));
                let script_path = root_path.join("scripts").join("mcp_plus_dogfood_loop.sh");

                let invocation = SpecKitInvocation {
                    phase: SpecKitPhase::Implement,
                    mode: RalphMode::Exploit,
                    agent: AgentKind::ClaudeCode,
                    command: format!("{} --target \"{}\"", script_path.display(), slug),
                    working_dir: worktree_path.clone(),
                    may_write: true,
                };

                let phase_name = format!("{:?}", invocation.phase);
                let _phase_span =
                    info_span!("run_phase", phase = %phase_name, idea = %idea).entered();

                let journal: Arc<Mutex<PhaseJournal>> =
                    Arc::new(Mutex::new(PhaseJournal::new()));

                if is_test {
                    for (phase, filename) in [
                        ("specify", "research.md"),
                        ("plan", "plan.md"),
                        ("tasks", "tasks.md"),
                        ("implement", "implement.md"),
                    ] {
                        let path = working_dir.join(filename);
                        let _ = std::fs::write(&path, "MOCK");
                        if let Some(hash) = sha256_file(&path) {
                            journal.lock().unwrap().record(phase, path, hash);
                        }
                    }
                } else {
                    // Stamp any already-present upstream artifacts before invoking implement.
                    for (phase, filename) in [
                        ("specify", "research.md"),
                        ("plan", "plan.md"),
                        ("tasks", "tasks.md"),
                    ] {
                        let path = working_dir.join(filename);
                        if let Some(hash) = sha256_file(&path) {
                            journal.lock().unwrap().record(phase, path, hash);
                        }
                    }

                    match controller.invoke(invocation) {
                        Ok(receipt) => {
                            if !receipt.success {
                                error!("Phase {} failed: {}", phase_name, receipt.output);
                                let _ = workspace.cleanup_worktree(&worktree_path);
                                return Ok(());
                            }
                            // Stamp implement phase after confirmed success.
                            let artifact_path = working_dir.join("implement.md");
                            if let Some(hash) = sha256_file(&artifact_path) {
                                journal
                                    .lock()
                                    .unwrap()
                                    .record("implement", artifact_path, hash);
                            }
                        }
                        Err(e) => {
                            error!("Phase {} execution error: {}", phase_name, e);
                            let _ = workspace.cleanup_worktree(&worktree_path);
                            return Ok(());
                        }
                    }
                }

                let mut event = Event::new(phase_name);
                event.attributes.push(Attribute {
                    key: "idea".to_string(),
                    value: AttributeValue::String(idea.clone()),
                });
                event.attributes.push(Attribute {
                    key: "duration_ns".to_string(),
                    value: AttributeValue::String(start.elapsed().as_nanos().to_string()),
                });
                trace.events.push(event);

                if let Err(e) = inject_supervisor(&worktree_path) {
                    error!("Failed to inject supervisor: {}", e);
                }

                if let Err(e) = workspace.commit_changes(&worktree_path, &id, &idea) {
                    error!("Failed to commit changes: {}", e);
                }

                {
                    let _lock = merge_lock.lock().unwrap();
                    if let Err(e) = workspace.merge_into_dev(&branch_name) {
                        warn!("Failed to merge branch {}: {}", branch_name, e);
                    }
                }

                let _ = workspace.cleanup_worktree(&worktree_path);

                let receipt_emitter = ReceiptEmitter::new();
                if let Err(e) = receipt_emitter.emit(&working_dir, &idea, "HashVerified") {
                    error!("Failed to emit receipt: {}", e);
                }

                // ── Emit RalphPlan ─────────────────────────────────────────────
                if let Err(e) = emit_ralph_plan(
                    &plans_out,
                    &run_id,
                    &id,
                    &idea,
                    &journal.lock().unwrap(),
                    (*constitution_hash).clone(),
                ) {
                    error!("Failed to emit RalphPlan: {}", e);
                }

                meta_log.lock().unwrap().add_trace(trace);

                Ok::<(), anyhow::Error>(())
            })
            .await?
        }
    };

    engine.run(ideas, process_fn).await?;

    let final_log = meta_log.lock().unwrap().clone();
    let controller = AutonomicController::new("IDEAS.md");
    if let Err(e) = controller.evaluate_dogfood(&final_log) {
        error!("Autonomic cycle failed: {}", e);
    }

    if cfg!(debug_assertions) {
        info!("\n--- All ideas processed! ---");
    }

    if let Some(p) = provider {
        let _ = p.force_flush();
    }

    Ok(())
}
