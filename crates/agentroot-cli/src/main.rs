//! Agentroot CLI
//!
//! Fast local search for your markdown knowledge base.

use agentroot_core::Database;
use anyhow::Result;
use clap::Parser;

mod app;
mod commands;
mod output;
mod progress;

use app::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    // Open database (use AGENTROOT_DB env var if set, otherwise use default)
    let db_path = std::env::var("AGENTROOT_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Database::default_path());
    let db = Database::open(&db_path)?;
    db.initialize()?;

    let result = match cli.command {
        Commands::Collection(args) => commands::collection::run(args, &db).await,
        Commands::Context(args) => commands::context::run(args, &db).await,
        Commands::Ls(args) => commands::ls::run(args, &db, cli.format).await,
        Commands::Get(args) => commands::get::run(args, &db, cli.format).await,
        Commands::MultiGet(args) => commands::get::run_multi(args, &db, cli.format).await,
        Commands::Status => commands::status::run(&db, cli.format).await,
        Commands::Update(args) => commands::update::run(args, &db, cli.verbose).await,
        Commands::Embed(args) => commands::embed::run(args, &db).await,
        Commands::Search(args) => commands::search::run_bm25(args, &db, cli.format).await,
        Commands::Vsearch(args) => commands::search::run_vector(args, &db, cli.format).await,
        Commands::Query(args) => commands::search::run_hybrid(args, &db, cli.format).await,
        Commands::Smart(args) => commands::search::run_smart(args, &db, cli.format).await,
        Commands::Cleanup => commands::cleanup::run(&db).await,
        Commands::Metadata(args) => commands::metadata::run(args, &db, cli.format).await,
        Commands::Mcp => agentroot_mcp::start_server(&db).await,
    };

    result
}
