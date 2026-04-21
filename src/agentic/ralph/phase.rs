use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub trait PhaseRunner: Send + Sync {
    fn run_phase(
        &self,
        id: &str,
        phase: &str,
        idea: &str,
        working_dir: &Path,
        is_test: bool,
        model: Option<String>,
        worktree_dir: Option<&Path>,
    ) -> Result<()>;
}

pub struct GeminiPhaseRunner;

impl GeminiPhaseRunner {
    pub fn new() -> Self {
        Self
    }
}

impl PhaseRunner for GeminiPhaseRunner {
    fn run_phase(
        &self,
        _id: &str,
        phase: &str,
        idea: &str,
        working_dir: &Path,
        is_test: bool,
        model: Option<String>,
        worktree_dir: Option<&Path>,
    ) -> Result<()> {
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
                 Analyze the system against DDS paradigms in @docs/DDS_THESIS.md and the \
                 UniverseOS Deterministic Operating Environment in @src/agentic/ralph/patterns/U64_ARCHITECTURE.md. \
                 Output a STORY.md with 'As a...', 'I want...', and 'So that...' sections.",
                idea
            ),
            "BacklogRefinement" => {
                let story_path = working_dir.join("STORY.md");
                format!(
                    "DDS SPRINT PLANNING: Given the STORY.md in @{}, define the formal \
                     Acceptance Criteria (AC) required for a DDS-grade implementation. \
                     Consult @docs/DDS_THESIS.md and @src/agentic/ralph/patterns/U64_ARCHITECTURE.md for \
                     UniverseOS architectural constraints (UInstruction, UDelta, UReceipt, UProjection). \
                     Output a detailed AC_CRITERIA.md report.",
                    story_path.display()
                )
            }
            "Implementation" => {
                let ac_path = working_dir.join("AC_CRITERIA.md");
                format!(
                    "DDS DEVELOPMENT PHASE: You are a DDS Synthesis Agent for UniverseOS. \
                     Implement the solution such that it meets all Acceptance Criteria in @{}. \
                     You MUST satisfy the following Definition of Done (DoD):\n \
                     1. ADMISSIBILITY: No unreachable states or unsafe panics. Every transition must be checked against the resident Data Plane.\n \
                     2. MINIMALITY: Satisfy MDL Φ(N) formula.\n \
                     3. PERFORMANCE: Zero-heap, branchless hot-path adhering to the T1 (<200ns) microkernel threshold for UTransitions.\n \
                     4. PROVENANCE: Every state motion must emit a UDelta and update the UReceipt rolling proof state.\n \
                     5. RIGOR: Include property-based tests (proptests) that assert both successful execution and expected failure/admissibility violations.\n \
                     Consult @docs/DDS_THESIS.md and @src/agentic/ralph/patterns/U64_ARCHITECTURE.md for UniverseOS law and C4 architecture. \
                     Modify files directly and output a DOD_VERIFICATION.md report.",
                    ac_path.display()
                )
            }
            _ => format!("Process idea: {}", idea),
        };

        let mut prompt = prompt;

        let target_agent = {
            let idea_lower = idea.to_lowercase();
            if phase == "Implementation" {
                if idea_lower.contains("q-table") || idea_lower.contains("rl ") || idea_lower.contains("sarsa") || idea_lower.contains("reinforcement") {
                    Some("@richard_sutton")
                } else if idea_lower.contains("wf-net") || idea_lower.contains("soundness") || idea_lower.contains("deadlock") || idea_lower.contains("liveness") {
                    Some("@dr_wil_van_der_aalst")
                } else if idea_lower.contains("replay") || idea_lower.contains("conformance") || idea_lower.contains("token") || idea_lower.contains("zero-heap") || idea_lower.contains("branchless") {
                    Some("@carl_adam_petri")
                } else if idea_lower.contains("autonomic") || idea_lower.contains("discovery") || idea_lower.contains("loop") {
                    Some("@arthur_ter_hofstede")
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(agent) = target_agent {
            prompt = format!("DELEGATION DIRECTIVE: You MUST delegate this entire task to the specialized sub-agent {}. Do not attempt to solve it yourself.\n\n{}", agent, prompt);
        }

        let max_attempts = if phase == "Implementation" { 3 } else { 1 };
        let mut last_output = String::new();

        for attempt in 1..=max_attempts {
            let mut cmd = Command::new("gemini");
            cmd.arg("-p").arg(&prompt);

            if let Some(ref m) = model {
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
                let _err = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Phase {} failed", phase));
            }

            last_output = String::from_utf8_lossy(&output.stdout).into_owned();

            if phase == "Implementation" {
                let dir = worktree_dir.unwrap_or(working_dir);
                let check_status = Command::new("cargo")
                    .arg("check")
                    .current_dir(dir)
                    .output()?;

                let test_status = Command::new("cargo")
                    .args(["test", "--lib"])
                    .current_dir(dir)
                    .output()?;

                if check_status.status.success() && test_status.status.success() {
                    break;
                } else {
                    let err_out = if !check_status.status.success() {
                        String::from_utf8_lossy(&check_status.stderr).into_owned()
                    } else {
                        String::from_utf8_lossy(&test_status.stdout).into_owned()
                    };

                    if attempt == max_attempts {
                        return Err(anyhow::anyhow!("Implementation failed verification: {}", err_out));
                    }

                    prompt = format!(
                        "The previous implementation failed verification. Fix the following errors. Do not revert to old code, fix the issues moving forward:\n\nERRORS:\n{}\n\nORIGINAL PROMPT:\n{}", 
                        err_out, prompt
                    );
                }
            } else {
                break;
            }
        }

        fs::write(output_file, last_output)?;
        Ok(())
    }
}
