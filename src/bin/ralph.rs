use dteam::dteam::orchestration::{Engine, EngineResult};
use dteam::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use opentelemetry::{global, KeyValue};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing::{debug, error, info, info_span, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn init_telemetry() -> anyhow::Result<SdkTracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint("http://localhost:4317")
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_attributes(vec![KeyValue::new(
            "service.name",
            "ralph-orchestrator",
        )]).build())
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = init_telemetry().ok(); // Gracefully handle if no OTel collector
    let _main_span = info_span!("ralph_main").entered();

    if cfg!(debug_assertions) {
        info!("--- Ralph Wiggum Loop: Rust Parallel Orchestrator ---");
    }

    let args: Vec<String> = std::env::args().collect();
    let is_test = args.contains(&"--test".to_string());

    let mut max_concurrency = 1;
    if let Some(pos) = args.iter().position(|a| a == "--concurrency") {
        if let Some(val) = args.get(pos + 1) {
            max_concurrency = val.parse::<usize>().unwrap_or(1);
        }
    }

    let mut model = Some("gemini-3-flash-preview".to_string()); 
    if let Some(pos) = args.iter().position(|a| a == "--model") {
        if let Some(val) = args.get(pos + 1) {
            model = Some(val.clone());
        }
    }

    if is_test {
        if cfg!(debug_assertions) {
            info!("!! TEST MODE ENABLED: Skipping LLM calls and using mock responses.");
        }
    }
    if cfg!(debug_assertions) {
        info!("!! CONCURRENCY LEVEL: {}", max_concurrency);
    }
    if let Some(m) = &model {
        if cfg!(debug_assertions) {
            info!("!! LLM MODEL: {}", m);
        }
    }

    let ideas_path = Path::new("IDEAS.md");
    if !ideas_path.exists() {
        if cfg!(debug_assertions) {
            info!("No IDEAS.md found. Creating a sample...");
        }
        fs::write(
            ideas_path,
            "1. Implement a basic health check endpoint\n2. Add logging to the autonomic cycle\n",
        )?;
    }

    let content = fs::read_to_string(ideas_path)?;
    let ideas: Vec<String> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    // Ensure dev branch exists
    ensure_dev_branch()?;

    // Instrumentation: Global EventLog
    let meta_log = Arc::new(Mutex::new(EventLog::default()));

    // Shared state for merging (must be serial)
    let merge_lock = Arc::new(Mutex::new(()));
    
    // Simple chunked concurrency
    for chunk in ideas.chunks(max_concurrency) {
        let mut handles = vec![];
        for (i, idea) in chunk.iter().cloned().enumerate() {
            let global_id = format!("{:03}", i + 1);
            let merge_lock = Arc::clone(&merge_lock);
            let model_clone = model.clone();
            let meta_log_clone = Arc::clone(&meta_log);

            let handle = thread::spawn(move || {
                let _span = info_span!("process_idea", idea = %idea, id = %global_id).entered();
                if let Err(e) =
                    process_idea(&global_id, &idea, is_test, model_clone, merge_lock, meta_log_clone)
                {
                    if cfg!(debug_assertions) {
                        error!("  !! Error processing idea '{}': {}", idea, e);
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for current batch to finish
        for handle in handles {
            let _ = handle.join();
        }
    }

    // --- Meta-Engine Cycle: Eating our own Dog Food ---
    if cfg!(debug_assertions) {
        info!("\n--- Process Complete. Running Meta-Engine (Dogfooding) ---");
    }
    let final_log = meta_log.lock().unwrap();

    let engine = Engine::builder().build();
    let result = engine.run(&final_log);

    if let EngineResult::Success(_net, manifest) = result {
        if cfg!(debug_assertions) {
            info!(
                "  >> Meta-Process Analysis Success. Model Canonical Hash: {}",
                manifest.model_canonical_hash
            );
        }
        if manifest.mdl_score > 0.0 {
            if cfg!(debug_assertions) {
                info!("  >> dteam identifies structural optimization potential. Injecting self-optimization task...");
            }
            let mut file = fs::OpenOptions::new().append(true).open("IDEAS.md")?;
            use std::io::Write;
            writeln!(
                file,
                "DDS-AUTO: Optimize Ralph Loop topology based on manifest hash {}",
                manifest.model_canonical_hash
            )?;
        }
    }

if cfg!(debug_assertions) {
    info!("\n--- All ideas processed! ---");
}

if let Some(p) = provider {
    p.force_flush();
}    Ok(())
}

fn process_idea(
    id: &str,
    idea: &str,
    is_test: bool,
    model: Option<String>,
    merge_lock: Arc<Mutex<()>>,
    meta_log: Arc<Mutex<EventLog>>,
) -> anyhow::Result<()> {
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

    let mut trace = Trace::default();
    trace.id = id.to_string();

    // Git branch management
    let branch_name = format!("wreckit/{}", slug);
    if cfg!(debug_assertions) {
        debug!("  >> Branch: {}", branch_name);
    }
    let worktree_path = working_dir.join("worktree");

    setup_worktree(&branch_name, &worktree_path)?;

    // Lifecycle phases
    let phases = vec!["UserStory", "BacklogRefinement", "Implementation"];
    for phase in phases {
        let start = Instant::now();
        let _span = info_span!("run_phase", phase = %phase, idea = %idea).entered();
        run_phase(
            id,
            phase,
            idea,
            &working_dir,
            is_test,
            model.clone(),
            Some(&worktree_path),
        )?;

        let mut event = Event::new(phase.to_string());
        event.attributes.push(Attribute {
            key: "idea".to_string(),
            value: AttributeValue::String(idea.to_string()),
        });
        event.attributes.push(Attribute {
            key: "duration_ns".to_string(),
            value: AttributeValue::String(start.elapsed().as_nanos().to_string()),
        });
        trace.events.push(event);
    }

    inject_supervisor(&worktree_path)?;

    commit_changes_in_worktree(&worktree_path, id, idea)?;

    // Merge into dev (SERIALIZED)
    {
        let _lock = merge_lock.lock().unwrap();
        if let Err(e) = merge_into_dev(&branch_name) {
            if cfg!(debug_assertions) {
                warn!("  !! Failed to merge branch {}: {}", branch_name, e);
            }
        }
    }

    cleanup_worktree(&worktree_path)?;
    
    // Add trace to meta log
    meta_log.lock().unwrap().add_trace(trace);

    Ok(())
}

fn ensure_dev_branch() -> anyhow::Result<()> {
    let status = Command::new("git")
        .args(["show-ref", "--verify", "refs/heads/dev"])
        .status()?;

    if !status.success() {
        if cfg!(debug_assertions) {
            info!("  !! dev branch missing. Creating from main...");
        }
        Command::new("git").args(["checkout", "-b", "dev"]).status()?;
        Command::new("git").args(["checkout", "main"]).status()?;
    }
    Ok(())
}

fn setup_worktree(branch: &str, path: &Path) -> anyhow::Result<()> {
    let branch_exists = Command::new("git")
        .args(["show-ref", "--verify", &format!("refs/heads/{}", branch)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !branch_exists {
        Command::new("git").args(["branch", branch]).status()?;
    }

    let status = Command::new("git")
        .args(["worktree", "add", path.to_str().unwrap(), branch])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create worktree"));
    }
    Ok(())
}

fn cleanup_worktree(path: &Path) -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        debug!("  >> Cleaning up worktree: {}", path.display());
    }
    Command::new("git")
        .args(["worktree", "remove", path.to_str().unwrap(), "--force"])
        .status()?;
    Ok(())
}

fn run_phase(
    _id: &str,
    phase: &str,
    idea: &str,
    working_dir: &Path,
    is_test: bool,
    model: Option<String>,
    worktree_dir: Option<&Path>,
) -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        debug!("  >> DDS Lifecycle: {} (Idea: {})", phase, idea);
    }
    let output_file = match phase {
        "UserStory" => working_dir.join("STORY.md"),
        "BacklogRefinement" => working_dir.join("AC_CRITERIA.md"),
        "Implementation" => working_dir.join("DOD_VERIFICATION.md"),
        _ => working_dir.join(format!("{}.md", phase.to_lowercase())),
    };

    if is_test {
        let mock_content = match phase {
            "UserStory" => format!("AS A: Process Architect\nI WANT: {}\nSO THAT: DDS paradigms are satisfied.", idea),
            "BacklogRefinement" => "ACCEPTANCE CRITERIA:\n1. Zero-heap verified\n2. Branchless logic confirmed.".to_string(),
            "Implementation" => "DEFINITION OF DONE:\n- [x] Code compiled\n- [x] MDL Score verified.\n- [x] Proptests implemented (Success/Failure cases).".to_string(),
            _ => "MOCK CONTENT".to_string(),
        };
        fs::write(output_file, mock_content)?;
        return Ok(());
    }

    let prompt = match phase {
        "UserStory" => format!(
            "DDS STORY GENERATION: Convert this idea into a formal User Story: '{}'. \
             Analyze the system against DDS paradigms in @docs/DDS_THESIS.md. \
             Output a STORY.md with 'As a...', 'I want...', and 'So that...' sections.",
            idea
        ),
        "BacklogRefinement" => {
            let story_path = working_dir.join("STORY.md");
            format!(
                "DDS SPRINT PLANNING: Given the STORY.md in @{}, define the formal \
                 Acceptance Criteria (AC) required for a DDS-grade implementation. \
                 Consult @docs/DDS_THESIS.md for constraints. \
                 Output a detailed AC_CRITERIA.md report.",
                story_path.display()
            )
        }
        "Implementation" => {
            let ac_path = working_dir.join("AC_CRITERIA.md");
            format!(
                "DDS DEVELOPMENT PHASE: You are a DDS Synthesis Agent. \
                 Implement the solution such that it meets all Acceptance Criteria in @{}. \
                 You MUST satisfy the following Definition of Done (DoD):\n \
                 1. ADMISSIBILITY: No unreachable states or unsafe panics.\n \
                 2. MINIMALITY: Satisfy MDL Φ(N) formula.\n \
                 3. PERFORMANCE: Zero-heap, branchless hot-path.\n \
                 4. PROVENANCE: Manifest updated.\n \
                 5. RIGOR: Include property-based tests (proptests) that assert both successful execution and expected failure/admissibility violations.\n \
                 Consult @docs/DDS_THESIS.md for formal definitions. \
                 Modify files directly and output a DOD_VERIFICATION.md report.",
                ac_path.display()
            )
        }
        _ => unreachable!(),
    };

    let mut cmd = Command::new("gemini");
    cmd.arg("-p").arg(prompt);

    if let Some(m) = model {
        cmd.arg("-m").arg(m);
    }

    if phase == "Implementation" {
        cmd.arg("--yolo");
    }

    if let Some(dir) = worktree_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        if cfg!(debug_assertions) {
            warn!("  !! {} Phase failed for idea '{}': {}", phase, idea, err);
        }
        return Err(anyhow::anyhow!("Phase {} failed", phase));
    }

    fs::write(output_file, output.stdout)?;
    Ok(())
}

fn inject_supervisor(working_dir: &Path) -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        debug!("  >> Injecting Supervisor Guardrails...");
    }
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

fn commit_changes_in_worktree(path: &Path, id: &str, idea: &str) -> anyhow::Result<()> {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["add", "."])
        .status()?;
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["commit", "-m", &format!("ralph({}): {}", id, idea)])
        .status()?;
    Ok(())
}

fn merge_into_dev(branch: &str) -> anyhow::Result<()> {
    Command::new("git").args(["checkout", "dev"]).status()?;
    let status = Command::new("git")
        .args(["merge", branch, "--no-edit"])
        .status()?;

    if !status.success() {
        Command::new("git").args(["merge", "--abort"]).status()?;
        return Err(anyhow::anyhow!("Merge conflict"));
    }
    Ok(())
}
