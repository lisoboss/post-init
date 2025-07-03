use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;

#[derive(Parser)]
#[command(name = "post-init")]
#[command(about = "A tool for post-initialization project setup and optimization")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize UV Python project with VCS versioning
    Uvinit {
        /// Target directory to search for pyproject.toml files
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Initialize Cargo Rust project
    Cargonew {
        /// Project name
        name: String,
        /// Project template
        #[arg(short, long, default_value = "bin")]
        template: String,
    },
    /// Initialize Tauri project
    Tuarinew {
        /// Project name
        name: String,
        /// Frontend framework
        #[arg(short, long, default_value = "vanilla")]
        frontend: String,
    },
    /// Show current configuration
    Config {
        /// Show config file path
        #[arg(short, long)]
        show_path: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Uvinit { path, yes } => {
            commands::uvinit::run_uvinit(&path, yes)?;
        }
        Commands::Cargonew { name, template } => {
            commands::cargonew::run_cargonew(&name, &template)?;
        }
        Commands::Tuarinew { name, frontend } => {
            commands::tuarinew::run_tuarinew(&name, &frontend)?;
        }
        Commands::Config { show_path } => {
            commands::config::show_config(show_path)?;
        }
    }

    Ok(())
}
