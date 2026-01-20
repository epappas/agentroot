# Example: Automated Metadata Output

This document shows exactly what automated metadata looks like in Agentroot.

## Sample Document

Let's say you have this Rust tutorial:

```markdown
# Getting Started with Rust

Rust is a systems programming language for beginners who want to learn 
safe, concurrent programming. Perfect for building web servers, command-line 
tools, and embedded systems.

## Key Features
- Memory safety without garbage collection
- Zero-cost abstractions  
- Fearless concurrency

This tutorial will teach you Rust fundamentals.
```

## Generated Metadata (Fallback Mode)

When you index this document **without an LLM model**, Agentroot generates metadata using intelligent heuristics:

```json
{
  "summary": "Rust is a systems programming language for beginners who want to learn safe, concurrent programming. Perfect for building web servers, command-line tools, and embedded systems.",
  
  "semantic_title": "Getting Started With Rust",
  
  "keywords": [
    "rust",
    "systems",
    "programming",
    "language",
    "beginners",
    "memory",
    "safety",
    "concurrency"
  ],
  
  "category": "tutorial",
  
  "intent": "Document from demo collection",
  
  "concepts": [
    "Rust",
    "Key",
    "Features"
  ],
  
  "difficulty": "intermediate",
  
  "suggested_queries": [
    "Getting Started With Rust"
  ]
}
```

### How It Works (Fallback Mode)

1. **Summary**: Extracted from first substantial paragraph
2. **Semantic Title**: Filename converted to title case
3. **Keywords**: Top terms by frequency (excluding stop words)
4. **Category**: Inferred from file extension and content patterns
5. **Concepts**: Capitalized terms (often proper nouns, technologies)
6. **Difficulty**: Default "intermediate" (can be enhanced with LLM)
7. **Suggested Queries**: Based on title

## Generated Metadata (With LLM)

When you index with an **LLM model** (e.g., llama-3.1-8b-instruct), you get much richer metadata:

```json
{
  "summary": "This comprehensive tutorial introduces Rust programming language, emphasizing its unique features like memory safety without garbage collection, zero-cost abstractions, and fearless concurrency. Designed for beginners, it covers fundamental concepts and practical applications including web servers, command-line tools, and embedded systems. The guide provides a solid foundation for learning safe concurrent programming in Rust.",
  
  "semantic_title": "Rust Programming Fundamentals: A Beginner's Guide to Safe Concurrent Development",
  
  "keywords": [
    "rust",
    "systems programming",
    "memory safety",
    "concurrent programming",
    "zero-cost abstractions",
    "beginner tutorial",
    "web servers",
    "embedded systems",
    "ownership",
    "borrowing"
  ],
  
  "category": "tutorial",
  
  "intent": "Teach beginners the fundamentals of Rust programming language with emphasis on memory safety and concurrent programming patterns. Provides foundational knowledge for building systems-level applications.",
  
  "concepts": [
    "Memory Safety",
    "Concurrent Programming",
    "Systems Programming",
    "Zero-cost Abstractions",
    "Ownership Model",
    "Rust Language",
    "Compiler Guarantees"
  ],
  
  "difficulty": "beginner",
  
  "suggested_queries": [
    "rust tutorial for beginners",
    "learn rust programming",
    "rust memory safety guide",
    "concurrent programming rust",
    "rust getting started",
    "systems programming tutorial"
  ]
}
```

### Improvements with LLM

- **Better Summary**: Understands context and purpose
- **Enhanced Title**: More descriptive and keyword-rich
- **Smarter Keywords**: Multi-word phrases, better relevance
- **Accurate Difficulty**: Correctly identifies "beginner"
- **Richer Intent**: Explains what problem it solves
- **Better Concepts**: Understands technical terms
- **More Queries**: Multiple ways users might search

## How to View Metadata

### CLI Commands

```bash
# Show metadata for a document
agentroot metadata show #abc123

# Output:
# Document: demo/rust-tutorial.md
# Doc ID: #abc123
#
# Summary:
#   This comprehensive tutorial introduces Rust programming...
#
# Category: tutorial
# Difficulty: beginner
# Keywords: rust, systems programming, memory safety, ...
#
# Concepts:
#   - Memory Safety
#   - Concurrent Programming
#   - Systems Programming
```

### In Search Results (MCP)

When searching via MCP tools, metadata is automatically included:

```json
{
  "results": [
    {
      "docid": "#abc123",
      "file": "demo/rust-tutorial.md",
      "title": "Getting Started with Rust",
      "score": 0.85,
      "summary": "This comprehensive tutorial introduces Rust...",
      "category": "tutorial",
      "difficulty": "beginner",
      "keywords": ["rust", "systems programming", "memory safety"]
    }
  ]
}
```

### Programmatic Access (Rust API)

```rust
use agentroot_core::{Database, SearchOptions};

let db = Database::open("path/to/index.sqlite")?;
db.initialize()?;

let options = SearchOptions {
    limit: 5,
    ..Default::default()
};

let results = db.search_fts("rust tutorial", &options)?;

for result in results {
    println!("Title: {}", result.title);
    
    if let Some(summary) = &result.llm_summary {
        println!("Summary: {}", summary);
    }
    
    if let Some(difficulty) = &result.llm_difficulty {
        println!("Difficulty: {}", difficulty);
    }
    
    if let Some(keywords) = &result.llm_keywords {
        println!("Keywords: {:?}", keywords);
    }
}
```

## Metadata in Database

All metadata is stored in the `documents` table:

```sql
SELECT 
    path,
    llm_summary,
    llm_title,
    llm_keywords,
    llm_category,
    llm_difficulty
FROM documents
WHERE collection = 'demo';
```

And automatically indexed for full-text search:

```sql
-- Metadata is searchable via FTS5
SELECT * FROM documents_fts 
WHERE documents_fts MATCH 'memory safety concurrent'
LIMIT 5;
```

## Comparison: Fallback vs LLM

| Field | Fallback Mode | With LLM |
|-------|--------------|----------|
| **Quality** | Good | Excellent |
| **Summary** | First paragraph | Comprehensive, contextual |
| **Keywords** | Term frequency | Multi-word phrases, semantic |
| **Difficulty** | Generic | Accurate analysis |
| **Intent** | Template | Specific purpose |
| **Concepts** | Capitalized words | Technical understanding |
| **Speed** | Instant (~13µs/doc) | Slower (~2-5s/doc, one-time) |
| **Accuracy** | 70-80% | 90-95% |

## When to Use Each Mode

### Use Fallback Mode When:
- You don't have an LLM model
- You need instant indexing
- Your documents have clear structure
- You have 10,000+ documents
- You want minimal overhead

### Use LLM Mode When:
- You want highest quality metadata
- You have time for initial indexing
- Your documents are complex or varied
- Accuracy is critical
- You have < 10,000 documents

## Custom Metadata (Future Feature)

Currently, metadata is automatically generated from content. To add custom metadata, you would:

1. **Extend the schema** (add custom columns)
2. **Use provider metadata** (pass via SourceItem)
3. **Post-process** (edit database directly)

Example future API:
```rust
// Future feature - not yet implemented
db.set_custom_metadata(
    "#abc123",
    json!({
        "author": "John Doe",
        "last_reviewed": "2024-01-20",
        "tags": ["official", "v1.0"]
    })
)?;
```

Currently, you can work around this by:
- Adding custom metadata to document content (frontmatter)
- Using collection names for categorization
- Using provider_config for collection-level metadata

## See Also

- [Metadata User Guide](metadata-user-guide.md)
- [Metadata Performance Report](metadata-performance.md)
- [MCP Server Documentation](mcp-server.md)
