import os
import sys

main_rs_content = """use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;

#[derive(Parser)]
#[command(name = "cargo xtask")]
#[command(about = "INSA anti-drift DX infrastructure", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the ultimate anti-drift diagnostic (fmt, clippy, test)
    Dx,
    /// Run memory layout checks
    Layout,
    /// Manage golden fixtures (verify/bless)
    Golden {
        #[arg(short, long)]
        bless: bool,
    },
    /// Replay .powl64 segments against ReferenceLawPath
    Replay {
        case: String,
    },
    /// Generate the full truthforge admission report
    Truthforge {
        case: String,
    },
    /// Explain a byte surface explicitly
    ExplainByte {
        family: String,
        value: u8,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dx => commands::dx::execute(),
        Commands::Layout => commands::layout::execute(),
        Commands::Golden { bless } => commands::golden::execute(bless),
        Commands::Replay { case } => commands::replay::execute(case),
        Commands::Truthforge { case } => commands::truthforge::execute(case),
        Commands::ExplainByte { family, value } => commands::explain::execute(family, value),
    }
}
"""

with open("/Users/sac/dteam/crates/insa/xtask/src/main.rs", "w") as f:
    f.write(main_rs_content)
    
mod_rs_content = """pub mod dx;
pub mod explain;
pub mod golden;
pub mod layout;
pub mod replay;
pub mod truthforge;
"""

with open("/Users/sac/dteam/crates/insa/xtask/src/commands/mod.rs", "w") as f:
    f.write(mod_rs_content)

def write_stub(name):
    stub_content = f"""use anyhow::Result;

pub fn execute() -> Result<()> {{
    println!("xtask {name} executed");
    Ok(())
}}
"""
    with open(f"/Users/sac/dteam/crates/insa/xtask/src/commands/{name}.rs", "w") as f:
        f.write(stub_content)

write_stub("dx")
write_stub("layout")

def write_stub_arg(name, arg):
    stub_content = f"""use anyhow::Result;

pub fn execute(_{arg}: {type(arg).__name__}) -> Result<()> {{
    println!("xtask {name} executed");
    Ok(())
}}
"""
    # Fix the type for bool vs String since type(arg).__name__ gives str not String
    rust_type = "bool" if isinstance(arg, bool) else "String"
    stub_content = f"""use anyhow::Result;

pub fn execute(_{arg}: {rust_type}) -> Result<()> {{
    println!("xtask {name} executed");
    Ok(())
}}
"""
    with open(f"/Users/sac/dteam/crates/insa/xtask/src/commands/{name}.rs", "w") as f:
        f.write(stub_content)

write_stub_arg("golden", True)
write_stub_arg("replay", "case")
write_stub_arg("truthforge", "case")

explain_content = """use anyhow::Result;

pub fn execute(_family: String, _value: u8) -> Result<()> {
    println!("xtask explain executed");
    Ok(())
}
"""
with open(f"/Users/sac/dteam/crates/insa/xtask/src/commands/explain.rs", "w") as f:
    f.write(explain_content)

