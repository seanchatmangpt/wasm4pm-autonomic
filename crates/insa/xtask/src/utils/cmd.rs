use anyhow::{anyhow, Result};
use std::process::Command;

pub fn run_cargo_cmd(args: &[&str], env: Option<(&str, &str)>) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(args);
    
    if let Some((k, v)) = env {
        cmd.env(k, v);
    }

    let status = cmd.status().map_err(|e| anyhow!("Failed to execute cargo process: {}", e))?;

    if !status.success() {
        return Err(anyhow!(
            "Command 'cargo {}' failed with status: {}",
            args.join(" "),
            status
        ));
    }

    Ok(())
}
