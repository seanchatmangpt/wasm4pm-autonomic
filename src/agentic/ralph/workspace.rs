use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;

pub trait WorkspaceManager {
    fn setup_worktree(&self, branch: &str, path: &Path) -> Result<()>;
    fn cleanup_worktree(&self, path: &Path) -> Result<()>;
    fn commit_changes(&self, path: &Path, id: &str, message: &str) -> Result<()>;
    fn merge_into_dev(&self, branch: &str) -> Result<()>;
    fn ensure_dev_branch(&self) -> Result<()>;
}

#[derive(Default)]
pub struct GitWorktreeManager;

impl WorkspaceManager for GitWorktreeManager {
    fn setup_worktree(&self, branch: &str, path: &Path) -> Result<()> {
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
            return Err(anyhow!("Failed to create worktree"));
        }
        Ok(())
    }

    fn cleanup_worktree(&self, path: &Path) -> Result<()> {
        #[cfg(debug_assertions)]
        tracing::debug!("  >> Cleaning up worktree: {}", path.display());

        Command::new("git")
            .args(["worktree", "remove", path.to_str().unwrap(), "--force"])
            .status()?;
        Ok(())
    }

    fn commit_changes(&self, path: &Path, id: &str, message: &str) -> Result<()> {
        let known_artifacts = ["research.md", "plan.md", "tasks.md", "implement.md"];
        let mut any_staged = false;
        for artifact in &known_artifacts {
            let artifact_path = path.join(artifact);
            if artifact_path.exists() {
                Command::new("git")
                    .arg("-C")
                    .arg(path)
                    .args(["add", "--", artifact])
                    .status()?;
                any_staged = true;
            }
        }
        if !any_staged {
            tracing::warn!(
                "commit_changes: no known artifacts in {}, skipping",
                path.display()
            );
            return Ok(());
        }
        Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["commit", "-m", &format!("ralph({}): {}", id, message)])
            .status()?;
        Ok(())
    }

    fn merge_into_dev(&self, branch: &str) -> Result<()> {
        Command::new("git").args(["checkout", "dev"]).status()?;
        let status = Command::new("git")
            .args(["merge", branch, "--no-edit"])
            .status()?;

        if !status.success() {
            Command::new("git").args(["merge", "--abort"]).status()?;
            return Err(anyhow!("Merge conflict"));
        }
        Ok(())
    }

    fn ensure_dev_branch(&self) -> Result<()> {
        let status = Command::new("git")
            .args(["show-ref", "--verify", "refs/heads/dev"])
            .status()?;

        if !status.success() {
            #[cfg(debug_assertions)]
            tracing::info!("  !! dev branch missing. Creating from main...");
            Command::new("git")
                .args(["checkout", "-b", "dev"])
                .status()?;
            Command::new("git").args(["checkout", "main"]).status()?;
        }
        Ok(())
    }
}
