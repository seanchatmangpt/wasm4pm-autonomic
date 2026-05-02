use anyhow::Result;
use crate::utils::cmd::run_cargo_cmd;

pub fn execute(bless: bool) -> Result<()> {
    if bless {
        println!(">>> INSA: Blessing Golden Fixtures (UPDATE_GOLDEN=1)");
        run_cargo_cmd(&["test", "--test", "golden_*", "--workspace"], Some(("UPDATE_GOLDEN", "1")))?;
        println!("[+] Golden fixtures successfully updated. Commit them securely.");
    } else {
        println!(">>> INSA: Verifying Canonical WireV1 Encoding (Golden Fixtures)");
        run_cargo_cmd(&["test", "--test", "golden_*", "--workspace"], None)?;
        println!("[+] Wire encodings match canonical golden byte signatures.");
    }
    Ok(())
}
