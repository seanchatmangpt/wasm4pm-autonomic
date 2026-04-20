use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde_json::json;

fn main() -> anyhow::Result<()> {
    println!("--- Ralph Wiggum Loop: Rust Orchestrator ---");

    let args: Vec<String> = std::env::args().collect();
    let is_test = args.contains(&"--test".to_string());

    if is_test {
        println!("!! TEST MODE ENABLED: Skipping LLM calls and using mock responses.");
    }

    let ideas_path = Path::new("IDEAS.md");
    if !ideas_path.exists() {
        println!("No IDEAS.md found. Creating a sample...");
        fs::write(ideas_path, "1. Implement a basic health check endpoint\n2. Add logging to the autonomic cycle\n")?;
    }

    let content = fs::read_to_string(ideas_path)?;
    let ideas: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    for (i, idea) in ideas.iter().enumerate() {
        let id = format!("{:03}", i + 1);
        let slug = idea.to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        
        let working_dir = PathBuf::from(".wreckit").join(format!("{}-{}", id, slug));
        fs::create_dir_all(&working_dir)?;

        println!("\n[Idea {}] Processing: {}", id, idea);

        // Git branch management
        let original_branch = Command::new("git")
            .args(["branch", "--show-current"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "main".to_string());

        let branch_name = format!("wreckit/{}", slug);
        println!("  >> Branch: {}", branch_name);

        // Check if branch exists
        let branch_exists = Command::new("git")
            .args(["show-ref", "--verify", &format!("refs/heads/{}", branch_name)])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if branch_exists {
            Command::new("git").args(["checkout", &branch_name]).status()?;
        } else {
            Command::new("git").args(["checkout", "-b", &branch_name]).status()?;
        }

        // 1. Research
        run_phase(&id, "Research", idea, &working_dir, is_test)?;

        // 2. Plan
        run_phase(&id, "Plan", idea, &working_dir, is_test)?;

        // 3. Inject Supervisor Hook
        inject_supervisor(&working_dir)?;

        // 4. Implement
        run_phase(&id, "Implement", idea, &working_dir, is_test)?;

        // 5. Commit
        commit_changes(&id, idea)?;

        // Return to original branch
        let _ = Command::new("git").args(["checkout", &original_branch]).status();
        }

    println!("\n--- All ideas processed! ---");
    Ok(())
}

fn run_phase(_id: &str, phase: &str, idea: &str, working_dir: &Path, is_test: bool) -> anyhow::Result<()> {
    println!("  >> Phase: {}", phase);
    let output_file = working_dir.join(format!("{}.md", phase.to_lowercase()));
    
    if is_test {
        let mock_content = match phase {
            "Research" => format!("MOCK RESEARCH for idea: {}\nPatterns: bitset, branchless\nFiles: src/lib.rs", idea),
            "Plan" => format!("MOCK PLAN for idea: {}\n1. Step A\n2. Step B", idea),
            "Implement" => format!("MOCK IMPLEMENTATION for idea: {}\nSuccess signal: <promise>COMPLETE</promise>", idea),
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
        },
        "Implement" => {
            let plan_path = working_dir.join("plan.md");
            format!(
                "IMPLEMENTATION DIRECTIVE: You are an autonomous agent. \
                 Execute the following implementation plan for the idea: '{}'. \
                 Modify the files directly on the filesystem and run standard checks. \
                 \n\n[PLAN]\n@{}", 
                idea, plan_path.display()
            )
        },
        _ => unreachable!(),
    };

    let mut cmd = Command::new("gemini");
    cmd.arg("--headless").arg("-p").arg(prompt);

    let output = cmd.output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        println!("  !! {} Phase failed: {}", phase, err);
        return Err(anyhow::anyhow!("Phase {} failed", phase));
    }

    fs::write(output_file, output.stdout)?;
    Ok(())
}

fn inject_supervisor(_working_dir: &Path) -> anyhow::Result<()> {
    println!("  >> Injecting Supervisor Guardrails...");
    
    let gemini_dir = Path::new(".gemini");
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
        serde_json::to_string_pretty(&settings)?
    )?;

    let hook_script = r#"#!/usr/bin/env bash
input=$(cat)
tool_name=$(echo "$input" | jq -r '.tool_name')
tool_input=$(echo "$input" | jq -r '.tool_input')

# 1. Secret Scanner
if [[ "$tool_input" == *"AKIA"* || "$tool_input" == *"sk-ant"* || "$tool_input" == *"AIza"* ]]; then
    echo '{"decision": "deny", "reason": "SECURITY ALERT: Potential API Key detected in payload."}'
    exit 0
fi

# 2. Syntax Validation for Rust (Experimental)
if [[ "$tool_name" == "write_file" || "$tool_name" == "replace" ]]; then
    file_path=$(echo "$tool_input" | jq -r '.file_path')
    if [[ "$file_path" == *.rs ]]; then
        # We can't easily perform a dry-run here without applying the tool logic manually.
        # For now, we block writes to sensitive files or known-bad patterns.
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

fn commit_changes(id: &str, idea: &str) -> anyhow::Result<()> {
    println!("  >> Committing changes...");
    Command::new("git").args(["add", "."]).status()?;
    Command::new("git")
        .args(["commit", "-m", &format!("ralph({}): {}", id, idea)])
        .status()?;
    Ok(())
}
