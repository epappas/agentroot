# AST-Aware Semantic Chunking

Agentroot uses tree-sitter to parse source code and extract semantic units (functions, classes, methods) as chunks for embedding. This produces higher quality embeddings compared to naive character-based chunking.

## Overview

Traditional chunking splits text at arbitrary character boundaries:

```
┌─────────────────────────────────────────────────────────────────┐
│ Character-based chunking (naive)                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  fn process_data(data: Vec<u8>) -> Result<Output> {             │
│      let parsed = parse_input(&data)?;          ← Chunk 1 ends  │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │
│      let validated = validate(parsed)?;         ← Chunk 2 starts│
│      transform(validated)                                        │
│  }                                                               │
│                                                                  │
│  Problem: Function split across chunks, context lost             │
└─────────────────────────────────────────────────────────────────┘
```

AST-aware chunking keeps semantic units intact:

```
┌─────────────────────────────────────────────────────────────────┐
│ AST-aware chunking (semantic)                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  /// Process input data and return transformed output            │
│  fn process_data(data: Vec<u8>) -> Result<Output> {             │
│      let parsed = parse_input(&data)?;                          │
│      let validated = validate(parsed)?;                         │
│      transform(validated)                                        │
│  }                                                               │
│  ↑                                                               │
│  Entire function is ONE chunk, including docstring               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Supported Languages

| Language | Parser | Semantic Nodes |
|----------|--------|----------------|
| Rust | tree-sitter-rust | `function_item`, `impl_item`, `struct_item`, `enum_item`, `trait_item`, `mod_item`, `type_item`, `const_item`, `static_item`, `macro_definition` |
| Python | tree-sitter-python | `function_definition`, `class_definition`, `decorated_definition` |
| JavaScript | tree-sitter-javascript | `function_declaration`, `class_declaration`, `method_definition`, `arrow_function`, `function_expression`, `export_statement` |
| TypeScript | tree-sitter-typescript | Same as JavaScript plus `interface_declaration`, `type_alias_declaration`, `enum_declaration` |
| Go | tree-sitter-go | `function_declaration`, `method_declaration`, `type_declaration`, `const_declaration`, `var_declaration` |

## Chunk Types

Each chunk is classified by its semantic type:

```rust
pub enum ChunkType {
    Function,   // Standalone functions
    Method,     // Methods within classes/impls
    Class,      // Class definitions
    Struct,     // Struct definitions (Rust, Go)
    Enum,       // Enum definitions
    Trait,      // Trait definitions (Rust)
    Interface,  // Interface definitions (TS, Go)
    Module,     // Module definitions
    Import,     // Import statements
    Text,       // Fallback for non-code
}
```

## Chunk Metadata

Each chunk includes rich metadata:

```rust
pub struct ChunkMetadata {
    /// Leading comments/docstrings above the chunk
    pub leading_trivia: String,

    /// Trailing comments on the same line
    pub trailing_trivia: String,

    /// Hierarchical path (e.g., "MyClass::my_method")
    pub breadcrumb: Option<String>,

    /// Source language
    pub language: Option<&'static str>,

    /// Line numbers (1-indexed)
    pub start_line: usize,
    pub end_line: usize,
}
```

### Breadcrumbs

Breadcrumbs provide hierarchical context:

```python
class UserService:
    def get_user(self, user_id: int):
        ...
```

The `get_user` method gets breadcrumb: `"UserService::get_user"`

### Leading Trivia

Comments and docstrings above a function are included:

```rust
/// Processes user input and returns validated data.
///
/// # Arguments
/// * `input` - Raw user input string
///
/// # Returns
/// Validated and sanitized user data
fn process_input(input: &str) -> Result<UserData> {
    // ...
}
```

The entire docstring is captured as `leading_trivia` and included in the chunk.

## How It Works

### 1. Language Detection

Files are detected by extension:

```rust
impl Language {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "js" | "jsx" => Some(Language::JavaScript),
            "ts" => Some(Language::TypeScript),
            "tsx" => Some(Language::TypeScriptTsx),
            "go" => Some(Language::Go),
            _ => None,
        }
    }
}
```

### 2. Tree-Sitter Parsing

Source code is parsed into an AST:

```rust
pub fn parse(source: &str, language: Language) -> Result<Tree> {
    let mut parser = Parser::new();
    let ts_language = get_tree_sitter_language(language);
    parser.set_language(&ts_language)?;
    parser.parse(source, None)
        .ok_or_else(|| Error::Parse("Failed to parse".into()))
}
```

### 3. Semantic Node Extraction

Each language strategy defines which node types are semantic boundaries:

```rust
// Rust semantic nodes
const RUST_SEMANTIC_NODES: &[&str] = &[
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "mod_item",
];

impl ChunkingStrategy for RustStrategy {
    fn semantic_node_types(&self) -> &[&str] {
        RUST_SEMANTIC_NODES
    }

    fn extract_chunks(&self, source: &str, root: Node) -> Result<Vec<SemanticChunk>> {
        // Walk AST and extract matching nodes
    }
}
```

### 4. Chunk Hash Computation

Each chunk gets a content-addressable hash:

```rust
pub fn compute_chunk_hash(text: &str, leading: &str, trailing: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(leading.as_bytes());
    hasher.update(text.as_bytes());
    hasher.update(trailing.as_bytes());
    hasher.finalize().to_hex()[..32].to_string()
}
```

The hash includes context (leading/trailing trivia) so that changes to comments also invalidate the cache.

## Oversized Chunk Handling

When a semantic unit exceeds the maximum chunk size (default: 2000 chars), it's split using striding:

```
┌─────────────────────────────────────────────────────────────────┐
│ Large function (5000 chars)                                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  fn very_large_function() {                                      │
│      // ... 5000 characters of code ...                         │
│  }                                                               │
│                                                                  │
│  Split into overlapping strides:                                 │
│                                                                  │
│  ┌──────────────────────┐                                       │
│  │ Stride 0 (0-2000)    │ breadcrumb: "very_large_function[0]"  │
│  └──────────────────────┘                                       │
│           ┌──────────────────────┐                              │
│           │ Stride 1 (1700-3700) │ 300 char overlap             │
│           └──────────────────────┘                              │
│                    ┌──────────────────────┐                     │
│                    │ Stride 2 (3400-5000) │                     │
│                    └──────────────────────┘                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Smart Boundary Detection

Strides prefer to break at natural boundaries:

1. Double newlines (`\n\n`) - paragraph breaks
2. Single newlines (`\n`) - line breaks
3. Spaces (` `) - word breaks
4. Character boundary (last resort)

```rust
fn find_safe_boundary(s: &str, index: usize) -> usize {
    // Search within 30% of stride size for natural breaks
    let search_start = index.saturating_sub(index * 30 / 100);

    // Prefer paragraph breaks
    if let Some(pos) = s[search_start..index].rfind("\n\n") {
        return search_start + pos + 2;
    }
    // Then line breaks
    if let Some(pos) = s[search_start..index].rfind('\n') {
        return search_start + pos + 1;
    }
    // Then word breaks
    if let Some(pos) = s[search_start..index].rfind(' ') {
        return search_start + pos + 1;
    }
    // Fall back to character boundary
    index
}
```

## Fallback Chunking

For unsupported languages, Agentroot falls back to character-based chunking with overlap:

```rust
pub fn chunk_by_chars(
    content: &str,
    chunk_size: usize,    // 2000 chars
    overlap: usize        // 300 chars (15%)
) -> Vec<Chunk>
```

This ensures all files can be indexed, even without AST parsing.

## Example Output

Given this Rust file:

```rust
//! User management module

use std::collections::HashMap;

/// Represents a user in the system.
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    /// Creates a new user with the given details.
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }

    /// Validates the user's email format.
    pub fn validate_email(&self) -> bool {
        self.email.contains('@')
    }
}
```

Produces these chunks:

| # | Type | Text (truncated) | Breadcrumb | Lines |
|---|------|------------------|------------|-------|
| 1 | Struct | `pub struct User { ... }` | `User` | 7-11 |
| 2 | Method | `impl User { ... }` | `User` | 13-23 |
| 3 | Method | `pub fn new(...) -> Self { ... }` | `User::new` | 15-17 |
| 4 | Method | `pub fn validate_email(&self) -> bool { ... }` | `User::validate_email` | 20-22 |

Note: The `impl` block is chunked as a whole, and individual methods within it are also extracted as separate chunks for fine-grained search.

## Configuration

### Maximum Chunk Size

```rust
let chunker = SemanticChunker::new()
    .with_max_chunk_chars(3000);  // Default: 2000
```

### Chunk Size Constants

```rust
pub const CHUNK_SIZE_CHARS: usize = 2000;
pub const CHUNK_OVERLAP_CHARS: usize = 300;  // 15% overlap
```

## Performance

- **Parsing**: ~1-5ms per file (depending on size)
- **Chunking**: ~0.1-1ms per file
- **Memory**: Tree-sitter uses a streaming parser with bounded memory

The AST parsing adds minimal overhead compared to the embedding computation, while significantly improving search quality.
