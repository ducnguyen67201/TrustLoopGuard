use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod guardrails;
mod http;
mod policy;
mod redteam;

#[derive(Parser)]
#[command(name = "tl", about = "TrustLoopGuard CLI", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Work with policy YAML locally or against tl-server.
    Policy {
        #[command(subcommand)]
        cmd: PolicyCmd,
    },
    /// Work with agents against tl-server.
    Agents {
        #[command(subcommand)]
        cmd: AgentsCmd,
    },
    /// Work with red-team jobs and regression gates against tl-server.
    Redteam {
        #[command(subcommand)]
        cmd: RedteamCmd,
    },
    /// Validate a policy YAML file.
    PolicyLint { path: PathBuf },
    /// Validate an agent profile YAML file.
    AgentLint { path: PathBuf },
}

#[derive(Subcommand)]
enum RedteamCmd {
    /// Work with durable regression cases and result summaries.
    Regressions {
        #[command(subcommand)]
        cmd: RegressionCmd,
    },
}

#[derive(Subcommand)]
enum RegressionCmd {
    /// Fail CI when a completed regression job exceeds result thresholds.
    Check {
        /// Regression job id returned by `tl redteam regressions run` or the API.
        job_id: String,
        /// Source job whose promoted cases were re-run.
        #[arg(long)]
        source_job_id: String,
        /// Restrict the check to one case key. Repeat for multiple keys.
        #[arg(long = "case-key")]
        case_keys: Vec<String>,
        /// Maximum cases to summarize when no case keys are provided.
        #[arg(long)]
        limit: Option<usize>,
        /// Allowed failed case count before the command exits non-zero.
        #[arg(long, default_value_t = 0)]
        max_failed: u32,
        /// Allowed missing case count before the command exits non-zero.
        #[arg(long, default_value_t = 0)]
        max_missing: u32,
        /// Allowed inconclusive case count before the command exits non-zero.
        #[arg(long, default_value_t = 0)]
        max_inconclusive: u32,
        /// Print the raw summary response as formatted JSON.
        #[arg(long)]
        json: bool,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
    /// List durable regression result snapshots newest-first.
    History {
        /// Optional source job filter.
        #[arg(long)]
        source_job_id: Option<String>,
        /// Optional regression job filter.
        #[arg(long)]
        job_id: Option<String>,
        /// Optional agent id filter.
        #[arg(long)]
        agent_id: Option<String>,
        /// Maximum snapshots to return.
        #[arg(long)]
        limit: Option<usize>,
        /// Print the raw history response as formatted JSON.
        #[arg(long)]
        json: bool,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// Manage guardrail policies attached to a specific agent.
    Guardrails {
        #[command(subcommand)]
        cmd: GuardrailsCmd,
    },
}

#[derive(Subcommand)]
enum GuardrailsCmd {
    /// Derive a guardrail policy set from the agent's stored
    /// `system_prompt` and persist each draft with `enabled=false`.
    /// Operators review the set and enable individual policies after.
    Generate {
        /// Agent identifier. Must already be registered via POST /v1/agents
        /// with a non-empty `system_prompt`.
        agent_id: String,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
    /// List guardrail policies owned by an agent.
    List {
        agent_id: String,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand)]
enum PolicyCmd {
    /// Validate a policy YAML file locally.
    Validate { path: PathBuf },
    /// Publish a policy YAML file to tl-server.
    Push {
        path: PathBuf,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
    /// Pull a policy YAML document from tl-server.
    Pull {
        policy_id: String,
        /// Destination YAML path.
        #[arg(short, long)]
        output: PathBuf,
        /// tl-server base URL. Defaults to TL_SERVER_URL or http://localhost:8080.
        #[arg(long)]
        url: Option<String>,
        /// Bearer API key. Defaults to TL_API_KEY when set.
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Policy { cmd } => policy::run(cmd).await,
        Cmd::Agents { cmd } => guardrails::run_agents(cmd).await,
        Cmd::Redteam { cmd } => redteam::run_redteam(cmd).await,
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
