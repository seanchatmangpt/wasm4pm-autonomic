use anyhow::Result;
use crate::utils::cmd::run_cargo_cmd;

pub fn execute() -> Result<()> {
    println!(">>> INSA: Enforcing Layout Gates");
    run_cargo_cmd(&["test", "--test", "layout_*", "--workspace"], None)
}
