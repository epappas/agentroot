# Agentroot Examples

Working code examples demonstrating how to use Agentroot as a library.

## Running Examples

From the workspace root:

```bash
# Run an example
cargo run -p agentroot-core --example basic_search

# With debug logging
RUST_LOG=debug cargo run -p agentroot-core --example basic_search
```

## Examples

### basic_search.rs

Demonstrates basic database operations and search:
- Opening a database
- Creating a collection
- Indexing content
- Performing BM25 search
- Retrieving documents

```bash
cargo run --example basic_search
```

### semantic_chunking.rs

Demonstrates AST-aware semantic chunking:
- Parsing code files with tree-sitter
- Extracting semantic units (functions, classes)
- Computing chunk hashes
- Understanding chunk metadata

```bash
cargo run --example semantic_chunking
```

### custom_index.rs

Demonstrates building a custom indexing pipeline:
- Scanning directories
- Parsing files
- Chunking content
- Computing hashes
- Batch insertion

```bash
cargo run --example custom_index
```

### github_provider.rs

Demonstrates using the GitHub provider for multi-source indexing:
- Fetching content from GitHub repositories
- Fetching specific files from GitHub
- Listing files with glob patterns
- Using provider metadata
- Indexing GitHub content in database

```bash
cargo run --example github_provider
```

**Note**: Requires internet connection. Set `GITHUB_TOKEN` environment variable for higher API rate limits.

### url_provider.rs

Demonstrates using the URL provider to index web content:
- Fetching content from HTTP/HTTPS URLs
- Title extraction from HTML and markdown
- Error handling for various HTTP status codes
- Timeout and redirect configuration
- Indexing web pages in database

```bash
cargo run --example url_provider
```

**Note**: Requires internet connection. Example includes comprehensive error handling demonstration.

### pdf_provider.rs

Demonstrates using the PDF provider to index PDF documents:
- Indexing single PDF files
- Scanning directories for PDF files
- Text extraction from PDFs
- Smart title extraction from content or filename
- Searching indexed PDF content

```bash
cargo run --example pdf_provider
```

**Note**: Create `example.pdf` or `./pdfs/` directory with PDF files to run the full example.

### sql_provider.rs

Demonstrates using the SQL provider to index database content:
- Creating a sample SQLite database
- Table-based indexing configuration
- Custom SQL query indexing
- Advanced queries with JOINs and filters
- Searching indexed database content

```bash
cargo run --example sql_provider
```

**Note**: Creates sample databases (`sample_data.db` and `example_sql.db`) for demonstration.

### custom_provider.rs

Demonstrates creating a custom provider implementation:
- Implementing the SourceProvider trait
- Async/await patterns for network operations
- JSON API integration example
- Caching strategies
- Error handling best practices

```bash
cargo run --example custom_provider
```

**Note**: Example template for building your own custom providers.

## Provider Summary

| Example | Provider Type | Use Case | Requirements |
|---------|--------------|----------|--------------|
| `github_provider.rs` | GitHub | Index GitHub repositories | Internet, optional GITHUB_TOKEN |
| `url_provider.rs` | URL | Index web pages | Internet connection |
| `pdf_provider.rs` | PDF | Index PDF documents | PDF files |
| `sql_provider.rs` | SQL | Index database content | SQLite database |
| `custom_provider.rs` | Custom | Template for custom sources | None (template) |

## Using Agentroot as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
agentroot-core = { path = "../agentroot/crates/agentroot-core" }
# Or from crates.io once published:
# agentroot-core = "0.1"
```

Basic usage:

```rust
use agentroot_core::{Database, SearchOptions};

#[tokio::main]
async fn main() -> agentroot_core::Result<()> {
    // Open database
    let db = Database::open("./agentroot.db")?;
    db.initialize()?;
    
    // Create collections from different sources
    
    // Local files
    db.add_collection("myproject", "/path/to/code", "**/*.rs", "file", None)?;
    
    // GitHub repository
    db.add_collection(
        "rust-docs",
        "https://github.com/rust-lang/rust",
        "**/*.md",
        "github",
        None,
    )?;
    
    // Web pages
    db.add_collection(
        "blog",
        "https://example.com/docs",
        "**/*.html",
        "url",
        None,
    )?;
    
    // PDF documents
    db.add_collection("pdfs", "/path/to/pdfs", "**/*.pdf", "pdf", None)?;
    
    // SQLite database
    db.add_collection(
        "articles",
        "/path/to/database.db",
        "",
        "sql",
        Some(r#"{"table":"articles","id_column":"id","title_column":"title","content_column":"body"}"#),
    )?;
    
    // Index all collections
    db.reindex_collection("myproject").await?;
    db.reindex_collection("rust-docs").await?;
    
    // Search across all collections
    let options = SearchOptions::default();
    let results = db.search_fts("error handling", &options)?;
    
    for result in results {
        println!("{}: {} ({}%)",
            result.display_path,
            result.title,
            (result.score * 100.0) as i32
        );
    }
    
    Ok(())
}
```

See individual example files for complete, runnable demonstrations of each provider.
