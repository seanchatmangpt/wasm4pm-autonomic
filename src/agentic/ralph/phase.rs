use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Gemini,
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RalphMode {
    Explore,
    Exploit,
    Verify,
    Reconcile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecKitPhase {
    Discover,
    Constitution,
    Specify,
    Clarify,
    Plan,
    RedTeam,
    Tasks,
    Implement,
    Verify,
    Reconcile,
    Receipt,
    Archive,
    Next,
}

#[derive(Debug, Clone)]
pub struct SpecKitInvocation {
    pub phase: SpecKitPhase,
    pub mode: RalphMode,
    pub agent: AgentKind,
    pub command: String,
    pub working_dir: PathBuf,
    pub may_write: bool,
}

pub struct PhaseReceipt {
    pub success: bool,
    pub output: String,
}

pub trait SpecKitRunner: Send + Sync {
    fn invoke(&self, invocation: SpecKitInvocation) -> Result<PhaseReceipt>;
}

pub struct SpeckitController;

impl Default for SpeckitController {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeckitController {
    pub fn new() -> Self {
        Self
    }

    fn execute_agent(&self, _agent: AgentKind, command: &str, dir: &Path) -> Result<PhaseReceipt> {
        // If it's a speckit command, we execute it via specify natively or via the dogfood loop
        let prog = "bash";
        let args = vec!["-c", command];

        tracing::info!("Executing agent subshell: {} {:?}", prog, args);
        let mut cmd = Command::new(prog);
        cmd.args(&args);
        cmd.current_dir(dir);

        let output = cmd
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run subshell {}: {}", prog, e))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(PhaseReceipt {
            success: output.status.success(),
            output: if output.status.success() {
                stdout
            } else {
                format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr)
            },
        })
    }
}

impl SpecKitRunner for SpeckitController {
    fn invoke(&self, invocation: SpecKitInvocation) -> Result<PhaseReceipt> {
        self.execute_agent(
            invocation.agent,
            &invocation.command,
            &invocation.working_dir,
        )
    }
}
