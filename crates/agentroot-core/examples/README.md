# Agentroot Examples

Working code examples demonstrating how to use Agentroot as a library.

## Running Examples

From the workspace root:

```bash
# Run an example
cargo run --example basic_search

# With debug logging
RUST_LOG=debug cargo run --example basic_search
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

fn main() -> agentroot_core::Result<()> {
    // Open database
    let db = Database::open("./agentroot.db")?;
    
    // Create collection
    db.create_collection("myproject", "/path/to/code", "**/*.rs")?;
    
    // Search
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
