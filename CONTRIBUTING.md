# Contributing to Agentroot

Thank you for your interest in contributing to Agentroot! This document provides guidelines for contributing.

## Code of Conduct

Be respectful and professional. We welcome contributions from everyone.

## Getting Started

### Prerequisites

- Rust 1.89.0 or later
- Git
- Familiarity with command-line tools

### Development Setup

1. Fork and clone the repository:

```bash
git clone https://github.com/yourusername/agentroot
cd agentroot
```

2. Build the project:

```bash
cargo build
```

3. Run tests:

```bash
cargo test
```

4. Try the CLI:

```bash
cargo run --bin agentroot -- --help
```

## Project Structure

Agentroot is a Cargo workspace with 4 crates:

```
agentroot/
├── crates/
│   ├── agentroot-core/    # Core library (search, indexing, db)
│   ├── agentroot-cli/     # Command-line interface
│   ├── agentroot-mcp/     # MCP server for AI assistants
│   └── agentroot-tui/     # Terminal UI (experimental)
├── docs/                   # Documentation
├── AGENTS.md              # AI agent guidelines (also coding standards)
├── LICENSE                # MIT license
└── README.md              # Project overview
```

## How to Contribute

### Reporting Bugs

Open an issue with:

1. Agentroot version (`agentroot --version`)
2. Operating system
3. Steps to reproduce
4. Expected vs actual behavior
5. Relevant logs (`RUST_LOG=debug agentroot <command>`)

### Suggesting Features

Open an issue describing:

1. The problem you're trying to solve
2. Why existing features don't work
3. Proposed solution (optional)
4. Examples of how it would be used

### Submitting Pull Requests

1. **Discuss first**: For large changes, open an issue to discuss before coding

2. **Create a branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Make changes**: Follow coding standards (see below)

4. **Test thoroughly**:
   ```bash
   cargo test
   cargo clippy --all-targets --all-features
   cargo fmt --check
   ```

5. **Commit with clear messages**:
   ```bash
   git commit -m "feat: add support for C language parsing"
   ```

6. **Push and create PR**:
   ```bash
   git push origin feature/your-feature-name
   ```

7. **Address review feedback**: Be responsive to code review comments

## Coding Standards

**See [AGENTS.md](AGENTS.md) for complete coding guidelines.** Key points:

### General Principles

- Write production-ready code only - **no TODO, FIXME, or placeholder comments**
- Keep functions under 50 lines
- Use explicit error handling (`Result<T>`, never `unwrap` in library code)
- Follow SOLID principles and keep code modular
- Prefer early returns over nested if-else

### Code Style

Run before committing:

```bash
cargo fmt
cargo clippy --all-targets --all-features
```

Import organization:

```rust
// 1. External crates
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

// 2. Workspace crates
use agentroot_core::{Database, SemanticChunk};

// 3. Standard library
use std::path::{Path, PathBuf};
use std::sync::Arc;

// 4. Crate-internal modules
use crate::error::{Result, AgentRootError};
```

### Naming Conventions

- `snake_case`: functions, variables, modules, file names
- `PascalCase`: types, traits, enum variants
- `SCREAMING_SNAKE_CASE`: constants
- `'a, 'b`: lifetime parameters

### Error Handling

Use `thiserror` for library errors:

```rust
#[derive(Debug, Error)]
pub enum AgentRootError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, AgentRootError>;
```

Use `anyhow` only in binaries (CLI).

### Documentation

Document all public APIs:

```rust
/// Searches the index using BM25 full-text search.
///
/// # Arguments
///
/// * `query` - Search query string
/// * `options` - Search options (limit, filters, etc.)
///
/// # Returns
///
/// Vector of search results sorted by relevance score.
///
/// # Errors
///
/// Returns `Error::Database` if the query fails.
pub fn search(&self, query: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
    // Implementation
}
```

### Testing

Write tests for new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_hash_stability() {
        let chunk = SemanticChunk::new("content");
        let hash1 = chunk.compute_hash();
        let hash2 = chunk.compute_hash();
        assert_eq!(hash1, hash2);
    }
}
```

Run specific tests:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_chunk_hash_stability

# Run tests for specific crate
cargo test -p agentroot-core
```

## Areas for Contribution

### Good First Issues

Look for issues labeled `good-first-issue`:

- Documentation improvements
- Test coverage
- Bug fixes
- Error message improvements

### High-Impact Areas

- **Language support**: Add tree-sitter parsers for new languages
- **Search quality**: Improve ranking algorithms
- **Performance**: Optimize hot paths (profiling required)
- **MCP integration**: Improve AI assistant integration
- **Testing**: Increase test coverage

### Adding Language Support

To add support for a new programming language:

1. Add tree-sitter parser dependency in `Cargo.toml`:
   ```toml
   tree-sitter-c = "0.23"
   ```

2. Add language enum variant in `crates/agentroot-core/src/index/ast_chunker/language.rs`:
   ```rust
   pub enum Language {
       // Existing languages...
       C,
   }
   ```

3. Add file extension mapping:
   ```rust
   match path.extension()?.to_str()? {
       // Existing mappings...
       "c" | "h" => Some(Language::C),
       _ => None,
   }
   ```

4. Create strategy in `crates/agentroot-core/src/index/ast_chunker/strategies/`:
   ```rust
   // c.rs
   pub struct CStrategy;
   
   impl ChunkingStrategy for CStrategy {
       fn semantic_node_types(&self) -> &[&str] {
           &["function_definition", "struct_specifier"]
       }
       
       fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
           // Implementation
       }
   }
   ```

5. Add tests and documentation

See existing strategies (Rust, Python, JavaScript) as examples.

## Git Workflow

### Commit Messages

Use conventional commits format:

- `feat: add new feature`
- `fix: fix bug in search`
- `refactor: extract helper function`
- `docs: update README`
- `test: add tests for chunking`

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation changes
- `refactor/description` - Code refactoring

### Before Committing

```bash
# Format code
cargo fmt

# Run linter
cargo clippy --all-targets --all-features

# Run tests
cargo test

# Stage specific files (not all)
git add path/to/file.rs

# Commit with clear message
git commit -m "feat: add C language support"
```

## Pull Request Process

1. **PR title**: Use conventional commit format
2. **Description**: Explain what and why
3. **Tests**: Include tests for new functionality
4. **Documentation**: Update docs if needed
5. **Changelog**: Add entry to CHANGELOG.md (if significant)
6. **Review**: Address all review comments
7. **Squash**: Maintainers may squash commits on merge

## Testing Guidelines

### Unit Tests

Place tests inline with code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration Tests

For CLI testing, use `assert_cmd`:

```rust
use assert_cmd::Command;

#[test]
fn test_cli_search() {
    let mut cmd = Command::cargo_bin("agentroot").unwrap();
    cmd.arg("search").arg("test");
    cmd.assert().success();
}
```

### Test Data

- Use minimal test data
- Include in code (not external files)
- Clean up in tests (use temp directories)

## Documentation Guidelines

### Code Documentation

- Public APIs must have doc comments
- Include examples where helpful
- Explain non-obvious behavior
- Document panics and errors

### User Documentation

Update relevant docs in `docs/`:

- Getting started guide
- CLI reference
- Architecture documentation
- Troubleshooting guide

### Keep Docs Accurate

Verify documentation matches code:

- Test examples actually work
- Update when behavior changes
- No placeholder or TODO sections

## Performance Considerations

When optimizing:

1. **Measure first**: Use `cargo flamegraph` or `hyperfine`
2. **Profile bottlenecks**: Don't guess what's slow
3. **Benchmark changes**: Prove improvement
4. **Document trade-offs**: Explain complexity vs speed choices

Hot paths to be aware of:

- AST parsing (per-file overhead)
- Embedding generation (bottleneck)
- Vector search (O(n) with corpus size)
- Database transactions (batch when possible)

## Release Process

For maintainers:

1. Update version in `Cargo.toml` (workspace)
2. Update CHANGELOG.md
3. Create git tag: `git tag -a v0.x.0 -m "Release v0.x.0"`
4. Push tag: `git push origin v0.x.0`
5. GitHub Actions builds and publishes release

## Questions?

- Open an issue for questions
- Discuss in existing issues
- Check documentation first

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
