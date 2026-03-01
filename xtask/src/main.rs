use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod gen_wit;
mod publish;
mod sort_deps;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build and maintenance tasks for flow-bot")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish workspace packages to crates.io in dependency order
    Publish {
        /// Run in dry-run mode (no actual publish)
        #[arg(long)]
        dry_run: bool,
        /// Allow dirty working directory
        #[arg(long)]
        allow_dirty: bool,
        /// Skip version verification
        #[arg(long)]
        no_verify: bool,
    },
    /// Sort dependencies alphabetically in all Cargo.toml files
    SortDeps {
        /// Check if dependencies are sorted (exit with error if not)
        #[arg(long)]
        check: bool,
    },
    /// Generate WIT bindings for the plugin SDK
    GenWit,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let project_root = find_project_root()?;

    match cli.command {
        Commands::Publish {
            dry_run,
            allow_dirty,
            no_verify,
        } => publish::run(&project_root, dry_run, allow_dirty, no_verify),
        Commands::SortDeps { check } => sort_deps::run(&project_root, check),
        Commands::GenWit => gen_wit::run(&project_root),
    }
}

fn find_project_root() -> anyhow::Result<PathBuf> {
    // Start from current directory and find the workspace root
    let mut dir = std::env::current_dir()?;

    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            // Check if it's a workspace root
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(dir);
            }
        }

        if !dir.pop() {
            anyhow::bail!("Could not find workspace root (no Cargo.toml with [workspace] found)");
        }
    }
}
