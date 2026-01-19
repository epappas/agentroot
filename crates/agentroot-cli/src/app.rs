//! CLI argument definitions

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agentroot")]
#[command(
    author,
    version,
    about = "Fast local search for your markdown knowledge base"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format
    #[arg(long, global = true, value_enum, default_value = "cli")]
    pub format: OutputFormat,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage collections
    Collection(CollectionArgs),

    /// Manage contexts
    Context(ContextArgs),

    /// List collections or files
    Ls(LsArgs),

    /// Get a document
    Get(GetArgs),

    /// Get multiple documents
    MultiGet(MultiGetArgs),

    /// Show index status
    Status,

    /// Update collections
    Update(UpdateArgs),

    /// Generate embeddings
    Embed(EmbedArgs),

    /// BM25 full-text search
    Search(SearchArgs),

    /// Vector similarity search
    Vsearch(SearchArgs),

    /// Hybrid search with reranking
    Query(SearchArgs),

    /// Database cleanup
    Cleanup,

    /// Start MCP server
    Mcp,
}

#[derive(Args)]
pub struct CollectionArgs {
    #[command(subcommand)]
    pub action: CollectionAction,
}

#[derive(Subcommand)]
pub enum CollectionAction {
    /// Add a new collection
    Add {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "**/*.md")]
        mask: String,
    },
    /// List all collections
    List,
    /// Remove a collection
    #[command(alias = "rm")]
    Remove { name: String },
    /// Rename a collection
    #[command(alias = "mv")]
    Rename { old_name: String, new_name: String },
}

#[derive(Args)]
pub struct ContextArgs {
    #[command(subcommand)]
    pub action: ContextAction,
}

#[derive(Subcommand)]
pub enum ContextAction {
    /// Add context to a path
    Add {
        #[arg(default_value = ".")]
        path: String,
        context: String,
    },
    /// List all contexts
    List,
    /// Check for missing contexts
    Check,
    /// Remove a context
    #[command(alias = "rm")]
    Remove { path: String },
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: Vec<String>,

    /// Number of results
    #[arg(short = 'n', default_value = "20")]
    pub limit: usize,

    /// Minimum score threshold
    #[arg(long, default_value = "0")]
    pub min_score: f64,

    /// Filter by collection
    #[arg(short, long)]
    pub collection: Option<String>,

    /// Show full document content
    #[arg(long)]
    pub full: bool,

    /// Include line numbers
    #[arg(long)]
    pub line_numbers: bool,

    /// Return all matches
    #[arg(long)]
    pub all: bool,
}

#[derive(Args)]
pub struct GetArgs {
    /// File path, docid (#abc123), or path:line
    pub file: String,

    /// Start from line number
    #[arg(long)]
    pub from: Option<usize>,

    /// Maximum lines to return
    #[arg(short)]
    pub l: Option<usize>,

    /// Include line numbers
    #[arg(long)]
    pub line_numbers: bool,
}

#[derive(Args)]
pub struct MultiGetArgs {
    /// Glob pattern or comma-separated list
    pub pattern: String,

    /// Maximum lines per file
    #[arg(short)]
    pub l: Option<usize>,

    /// Maximum file size in bytes
    #[arg(long, default_value = "10240")]
    pub max_bytes: usize,

    /// Include line numbers
    #[arg(long)]
    pub line_numbers: bool,
}

#[derive(Args)]
pub struct LsArgs {
    /// Collection name or path
    pub path: Option<String>,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Run git pull before updating
    #[arg(long)]
    pub pull: bool,
}

#[derive(Args)]
pub struct EmbedArgs {
    /// Force re-embedding of all documents
    #[arg(short, long)]
    pub force: bool,

    /// Path to embedding model (GGUF file)
    #[arg(short, long)]
    pub model: Option<std::path::PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Cli,
    Json,
    Csv,
    Md,
    Xml,
    Files,
}
