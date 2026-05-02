use anyhow::Result;
use crate::utils::cmd::run_cargo_cmd;

pub fn execute() -> Result<()> {
    println!(">>> INSA DX: Running format check...");
    run_cargo_cmd(&["fmt", "--all", "--", "--check"], None)?;

    println!("\n>>> INSA DX: Running structural lints...");
    run_cargo_cmd(&["clippy", "--all-targets", "--workspace", "--", "-D", "warnings"], None)?;

    println!("\n>>> INSA DX: Running unit tests...");
    run_cargo_cmd(&["test", "--lib", "--workspace"], None)?;

    println!("\n>>> INSA DX: Running property tests...");
    run_cargo_cmd(&["test", "--test", "prop_*", "--workspace"], None)?;

    println!("\n>>> INSA DX: Running compile-fail tests...");
    run_cargo_cmd(&["test", "--test", "compile_fail_*", "--workspace"], None)?;

    println!("\n>>> INSA DX: Running golden wire tests...");
    run_cargo_cmd(&["test", "--test", "golden_*", "--workspace"], None)?;

    println!("\n>>> INSA DX: Running layout gates...");
    run_cargo_cmd(&["test", "--test", "layout_*", "--workspace"], None)?;

    println!("\n>>> INSA DX: Running end-to-end JTBD cases...");
    run_cargo_cmd(&["test", "--test", "jtbd_*", "--workspace"], None)?;

    println!("\n>>> INSA DX: Running benchmark smoke test...");
    run_cargo_cmd(&["bench", "--no-run", "--workspace"], None)?;

    println!("\n[+] INSA DX Gate Passed: The project is locally sane and anti-drift protocols hold.\n");
    Ok(())
}
