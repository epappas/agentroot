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

    /// Smart natural language search with auto fallback
    Smart(SearchArgs),

    /// Database cleanup
    Cleanup,

    /// Manage LLM-generated metadata
    Metadata(MetadataArgs),

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
        /// Path to local directory or URL (e.g., https://github.com/owner/repo)
        path: PathBuf,

        /// Collection name (defaults to directory/repo name)
        #[arg(long)]
        name: Option<String>,

        /// Glob pattern to match files (e.g., **/*.rs, **/*.{md,txt})
        #[arg(long, default_value = "**/*.md")]
        mask: String,

        /// Provider type: file, github, url, etc. (defaults to 'file')
        #[arg(long, default_value = "file")]
        provider: String,

        /// Provider-specific configuration as JSON (e.g., {"github_token": "ghp_..."})
        #[arg(long)]
        config: Option<String>,
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
    #[arg(long, alias = "from-line")]
    pub from: Option<usize>,

    /// Maximum lines to return
    #[arg(short, long = "max-lines")]
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
    #[arg(short, long = "max-lines")]
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
}

#[derive(Args)]
pub struct MetadataArgs {
    #[command(subcommand)]
    pub action: MetadataAction,
}

#[derive(Subcommand)]
pub enum MetadataAction {
    /// Regenerate LLM metadata for a collection
    Refresh {
        /// Collection name (or --all for all collections)
        collection: Option<String>,

        /// Regenerate for all collections
        #[arg(long)]
        all: bool,

        /// Document ID or path to refresh (single document)
        #[arg(long)]
        doc: Option<String>,

        /// Force regeneration even if cached
        #[arg(short, long)]
        force: bool,
    },
    /// Show all metadata for a document (LLM + user)
    Show {
        /// Document ID (#abc123) or path
        docid: String,
    },
    /// Add user metadata to a document
    Add {
        /// Document ID (#abc123) or path
        docid: String,

        /// Text metadata (key=value)
        #[arg(long)]
        text: Vec<String>,

        /// Integer metadata (key=value)
        #[arg(long)]
        integer: Vec<String>,

        /// Float metadata (key=value)
        #[arg(long)]
        float: Vec<String>,

        /// Boolean metadata (key=value)
        #[arg(long)]
        boolean: Vec<String>,

        /// DateTime metadata (key=value)
        #[arg(long)]
        datetime: Vec<String>,

        /// Tags metadata (key=value1,value2,...)
        #[arg(long)]
        tags: Vec<String>,

        /// Enum metadata (key=value:option1,option2,...)
        #[arg(long)]
        enum_value: Vec<String>,

        /// Qualitative metadata (key=value:scale1,scale2,...)
        #[arg(long)]
        qualitative: Vec<String>,

        /// Quantitative metadata (key=value:unit)
        #[arg(long)]
        quantitative: Vec<String>,
    },
    /// Get user metadata for a document
    Get {
        /// Document ID (#abc123) or path
        docid: String,
    },
    /// Remove specific user metadata fields from a document
    Remove {
        /// Document ID (#abc123) or path
        docid: String,

        /// Field names to remove
        fields: Vec<String>,
    },
    /// Clear all user metadata from a document
    Clear {
        /// Document ID (#abc123) or path
        docid: String,
    },
    /// Query documents by user metadata
    Query {
        /// Metadata filter expression (e.g., "tags:contains=rust" or "score:gt=10")
        filter: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
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
