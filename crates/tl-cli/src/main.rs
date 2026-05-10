use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tl", about = "TrustLoopGuard CLI", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Validate a policy YAML file.
    PolicyLint { path: PathBuf },
    /// Validate an agent profile YAML file.
    AgentLint { path: PathBuf },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::PolicyLint { path } => {
            let src = std::fs::read_to_string(&path)?;
            let policy = tl_policy::load_str(&src)?;
            println!("ok: policy `{}` parsed", policy.id);
            Ok(())
        }
        Cmd::AgentLint { path } => {
            let src = std::fs::read_to_string(&path)?;
            let profile = tl_policy::load_agent_str(&src)?;
            println!(
                "ok: agent `{}` ({}) parsed",
                profile.agent_id, profile.display_name
            );
            Ok(())
        }
    }
}
