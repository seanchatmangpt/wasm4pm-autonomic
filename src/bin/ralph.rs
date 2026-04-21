use dteam::agentic::ralph::{
    AutonomicController, ExecutionEngine, GeminiPhaseRunner, GitWorktreeManager, PhaseRunner,
    WorkspaceManager,
};
use dteam::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use opentelemetry::{global, KeyValue};
use opentelemetry::trace::TracerProvider as _;
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

<<<<<<< HEAD
    Ok(provider)
=======
    for (i, idea) in ideas.iter().enumerate() {
        let id = format!("{:03}", i + 1);
        let slug = idea
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        let working_dir = PathBuf::from(".wreckit").join(format!("{}-{}", id, slug));
        fs::create_dir_all(&working_dir)?;

        println!("\n[Idea {}] Processing: {}", id, idea);

        // Git branch management
        let branch_name = format!("wreckit/{}", slug);
        let worktree_path = working_dir.join("worktree");
        println!("  >> Branch: {}", branch_name);
        println!("  >> Worktree: {}", worktree_path.display());

        // 1. Setup Branch and Worktree
        setup_worktree(&branch_name, &worktree_path)?;

        // 2. Research (Run inside worktree)
        run_phase(
            &id,
            "Research",
            idea,
            &working_dir,
            is_test,
            Some(&worktree_path),
        )?;

        // 3. Plan (Run inside worktree)
        run_phase(
            &id,
            "Plan",
            idea,
            &working_dir,
            is_test,
            Some(&worktree_path),
        )?;

        // 4. Inject Supervisor Hook (In the worktree)
        inject_supervisor(&worktree_path)?;

        // 5. Implement (Run inside worktree)
        run_phase(
            &id,
            "Implement",
            idea,
            &working_dir,
            is_test,
            Some(&worktree_path),
        )?;

        // 6. Commit inside worktree
        commit_changes_in_worktree(&worktree_path, &id, idea)?;

        // 7. Merge into dev
        merge_into_dev(&branch_name)?;

        // 8. Cleanup Worktree
        cleanup_worktree(&worktree_path)?;
    }

    println!("\n--- All ideas processed! ---");
    Ok(())
}

fn ensure_dev_branch() -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["show-ref", "--verify", "refs/heads/dev"])
        .status()?;

    if !output.success() {
        println!("  !! dev branch missing. Creating from main...");
        Command::new("git")
            .args(["checkout", "-b", "dev"])
            .status()?;
        Command::new("git").args(["checkout", "main"]).status()?;
    }
    Ok(())
}

fn setup_worktree(branch: &str, path: &Path) -> anyhow::Result<()> {
    // Check if branch exists
    let branch_exists = Command::new("git")
        .args(["show-ref", "--verify", &format!("refs/heads/{}", branch)])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !branch_exists {
        Command::new("git").args(["branch", branch]).status()?;
    }

    // Add worktree
    let status = Command::new("git")
        .args(["worktree", "add", path.to_str().unwrap(), branch])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create worktree"));
    }
    Ok(())
}

fn cleanup_worktree(path: &Path) -> anyhow::Result<()> {
    println!("  >> Cleaning up worktree...");
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
    worktree_dir: Option<&Path>,
) -> anyhow::Result<()> {
    println!("  >> Phase: {}", phase);
    let output_file = working_dir.join(format!("{}.md", phase.to_lowercase()));

    if is_test {
        let mock_content = match phase {
            "Research" => format!(
                "MOCK RESEARCH for idea: {}\nPatterns: bitset, branchless\nFiles: src/lib.rs",
                idea
            ),
            "Plan" => format!("MOCK PLAN for idea: {}\n1. Step A\n2. Step B", idea),
            "Implement" => format!(
                "MOCK IMPLEMENTATION for idea: {}\nSuccess signal: <promise>COMPLETE</promise>",
                idea
            ),
            _ => "MOCK CONTENT".to_string(),
        };
        fs::write(output_file, mock_content)?;
        return Ok(());
    }

    let prompt = match phase {
        "Research" => format!(
            "RESEARCH DIRECTIVE: Research the codebase for the following idea: '{}'. \
             Analyze existing patterns, file paths, and integration points. \
             Output a detailed research.md report.",
            idea
        ),
        "Plan" => {
            let research_path = working_dir.join("research.md");
            format!(
                "PLANNING DIRECTIVE: Given the following idea: '{}' and the research findings below, \
                 write a detailed implementation plan. \
                 [RESEARCH]\n@{}\n\nOutput a detailed plan.md report.", 
                idea, research_path.display()
            )
        }
        "Implement" => {
            let plan_path = working_dir.join("plan.md");
            format!(
                "IMPLEMENTATION DIRECTIVE: You are an autonomous agent. \
                 Execute the following implementation plan for the idea: '{}'. \
                 Modify the files directly on the filesystem and run standard checks. \
                 \n\n[PLAN]\n@{}",
                idea,
                plan_path.display()
            )
        }
        _ => unreachable!(),
    };

    let mut cmd = Command::new("gemini");
    cmd.arg("-p").arg(prompt);

    if phase == "Implement" {
        cmd.arg("--yolo");
    }

    if let Some(dir) = worktree_dir {
        cmd.current_dir(dir);
    }

    let output = cmd.output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        println!("  !! {} Phase failed: {}", phase, err);
        return Err(anyhow::anyhow!("Phase {} failed", phase));
    }

    fs::write(output_file, output.stdout)?;
    Ok(())
>>>>>>> wreckit/cryptographic-execution-provenance-enhance-executionmanifest-with-full-h-l-π-h-n-hashing
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = init_telemetry().ok();
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

    if is_test {
        if cfg!(debug_assertions) {
            info!("!! TEST MODE ENABLED: Skipping LLM calls and using mock responses.");
        }
    }

    let ideas_path = Path::new("IDEAS.md");
    if !ideas_path.exists() {
        fs::write(
            ideas_path,
            "1. Implement a basic health check endpoint
2. Add logging to the autonomic cycle
",
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

    let workspace_manager = GitWorktreeManager::default();
    workspace_manager.ensure_dev_branch()?;

    let meta_log = Arc::new(Mutex::new(EventLog::default()));
    let merge_lock = Arc::new(Mutex::new(()));

    let engine = ExecutionEngine::new(max_concurrency);

    let meta_log_clone = Arc::clone(&meta_log);
    let merge_lock_clone = Arc::clone(&merge_lock);

    let process_fn = move |id: String, idea: String| {
        let model = model.clone();
        let meta_log = Arc::clone(&meta_log_clone);
        let merge_lock = Arc::clone(&merge_lock_clone);

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
                    info!("
[Idea {}] Processing: {}", id, idea);
                }

                let mut trace = Trace::default();
                trace.id = id.clone();

                let branch_name = format!("wreckit/{}", slug);
                let worktree_path = working_dir.join("worktree");

                let workspace = GitWorktreeManager::default();
                if let Err(e) = workspace.setup_worktree(&branch_name, &worktree_path) {
                    error!("Failed to setup worktree: {}", e);
                    return Ok::<(), anyhow::Error>(());
                }

                let runner = GeminiPhaseRunner::new();
                let phases = vec!["UserStory", "BacklogRefinement", "Implementation"];
                for phase in phases {
                    let start = Instant::now();
                    let _phase_span = info_span!("run_phase", phase = %phase, idea = %idea).entered();
                    if let Err(e) = runner.run_phase(
                        &id,
                        phase,
                        &idea,
                        &working_dir,
                        is_test,
                        model.clone(),
                        Some(&worktree_path),
                    ) {
                        error!("Phase {} failed: {}", phase, e);
                        let _ = workspace.cleanup_worktree(&worktree_path);
                        return Ok(());
                    }

                    let mut event = Event::new(phase.to_string());
                    event.attributes.push(Attribute {
                        key: "idea".to_string(),
                        value: AttributeValue::String(idea.clone()),
                    });
                    event.attributes.push(Attribute {
                        key: "duration_ns".to_string(),
                        value: AttributeValue::String(start.elapsed().as_nanos().to_string()),
                    });
                    trace.events.push(event);
                }

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
                meta_log.lock().unwrap().add_trace(trace);

                Ok::<(), anyhow::Error>(())
            }).await?
        }
    };

    engine.run(ideas, process_fn).await?;

    let final_log = meta_log.lock().unwrap().clone();
    let controller = AutonomicController::new("IDEAS.md");
    if let Err(e) = controller.evaluate_dogfood(&final_log) {
        error!("Autonomic cycle failed: {}", e);
    }

    if cfg!(debug_assertions) {
        info!("
--- All ideas processed! ---");
    }

    if let Some(p) = provider {
        let _ = p.force_flush();
    }

    Ok(())
}
