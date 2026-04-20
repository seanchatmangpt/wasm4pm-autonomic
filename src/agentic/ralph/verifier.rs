use std::path::Path;
use std::process::Command;
use anyhow::{Result, anyhow};

pub trait DoDVerifier {
    fn verify(&self, working_dir: &Path) -> Result<()>;
}

#[derive(Default)]
pub struct CargoVerifier;

impl DoDVerifier for CargoVerifier {
    fn verify(&self, working_dir: &Path) -> Result<()> {
        let check_status = Command::new("cargo")
            .arg("check")
            .current_dir(working_dir)
            .output()?;

        let test_status = Command::new("cargo")
            .args(["test", "--lib"])
            .current_dir(working_dir)
            .output()?;

        if check_status.status.success() && test_status.status.success() {
            Ok(())
        } else {
            let err_out = if !check_status.status.success() {
                String::from_utf8_lossy(&check_status.stderr).into_owned()
            } else {
                String::from_utf8_lossy(&test_status.stdout).into_owned()
            };
            Err(anyhow!("Verification failed: {}", err_out))
        }
    }
}
