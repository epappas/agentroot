# Custom Metadata Implementation Plan

## Vision

Transform Agentroot into a **universal knowledge index** supporting:
- Multi-source data ingestion (docs, code, SQL, time-series, calendars, data lakes)
- Typed user metadata (datetime, numeric, qualitative, quantitative, tags, enums)
- Hybrid metadata: LLM-generated + user-provided
- Metadata-aware embeddings
- Flexible batch and incremental operations

## Current Status

### ✅ Completed (Phase 1)

1. **Metadata Type System** (`db/metadata.rs`)
   - `MetadataValue` enum with 9 types:
     - Text, Integer, Float, Boolean
     - DateTime (ISO 8601)
     - Tags (Vec<String>)
     - Enum (validated values)
     - Qualitative (scales like "high/medium/low")
     - Quantitative (value + unit)
     - Json (nested data)
   
2. **Metadata Builder API**
   ```rust
   let metadata = MetadataBuilder::new()
       .text("author", "John Doe")
       .integer("version", 2)
       .datetime_now("last_reviewed")
       .tags("labels", vec!["rust", "official"])
       .quantitative("size", 1024.0, "KB")
       .enum_value("status", "published", vec!["draft", "published"])?
       .build();
   ```

3. **Metadata Filtering**
   - TextEq, TextContains
   - IntegerEq, IntegerGt, IntegerLt, IntegerRange
   - FloatGt, FloatLt, FloatRange
   - BooleanEq
   - DateTimeAfter, DateTimeBefore, DateTimeRange
   - TagsContain, TagsContainAll, TagsContainAny
   - EnumEq
   - Exists, And, Or, Not

4. **Database Operations** (`db/user_metadata.rs`)
   - `add_metadata(docid, metadata)` - Add/merge user metadata
   - `get_metadata(docid)` - Retrieve user metadata
   - `remove_metadata_fields(docid, fields)` - Remove specific fields
   - `clear_metadata(docid)` - Clear all user metadata
   - `find_by_metadata(filter, limit)` - Query by metadata
   - `list_with_metadata(limit)` - List all docs with metadata

### 🚧 Remaining Work (Phase 2)

1. **Schema Migration**
   - Add `user_metadata TEXT` column to `documents` table
   - Create migration from schema v4 to v5
   - Update `CREATE_TABLES` SQL

2. **CLI Commands**
   ```bash
   # Add metadata
   agentroot metadata add #abc123 \
     --text author="John Doe" \
     --tags labels=rust,tutorial \
     --integer version=2 \
     --datetime last_reviewed=now
   
   # Get metadata
   agentroot metadata get #abc123
   
   # Query by metadata
   agentroot metadata query --filter "integer:version>1"
   
   # Remove fields
   agentroot metadata remove #abc123 author version
   
   # Clear all
   agentroot metadata clear #abc123
   ```

3. **Search Integration**
   - Include user metadata in `SearchResult`
   - Combine LLM metadata + user metadata in results
   - Add metadata filters to `SearchOptions`
   - Support metadata-based ranking boosts

4. **Provider Integration**
   - Allow providers to supply initial metadata
   - `SourceItem` should carry metadata
   - Merge provider metadata with LLM metadata

5. **Metadata-Aware Embeddings**
   - Generate embeddings that include metadata context
   - Separate embeddings for: content, content+LLM metadata, content+all metadata
   - Allow searching by metadata similarity

6. **MCP Tool Updates**
   - Add `metadata` field to search results
   - Support metadata filters in search/vsearch/query
   - New MCP tools: `metadata_add`, `metadata_get`, `metadata_query`

7. **Documentation**
   - User guide for custom metadata
   - API examples for all metadata types
   - Migration guide from v4 to v5

## API Design

### Rust API

```rust
use agentroot_core::{Database, MetadataBuilder};

// Add metadata
let metadata = MetadataBuilder::new()
    .text("author", "Alice")
    .tags("labels", vec!["rust", "tutorial"])
    .datetime_now("created")
    .build();

db.add_metadata("#abc123", &metadata)?;

// Get metadata
let metadata = db.get_metadata("#abc123")?.unwrap();
println!("Author: {:?}", metadata.get("author"));

// Query by metadata
use agentroot_core::MetadataFilter;

let filter = MetadataFilter::And(vec![
    MetadataFilter::TagsContain("labels".to_string(), "rust".to_string()),
    MetadataFilter::DateTimeAfter("created".to_string(), "2024-01-01T00:00:00Z".to_string()),
]);

let docids = db.find_by_metadata(&filter, 10)?;
```

### CLI Examples

```bash
# Time-series data point
agentroot metadata add #abc123 \
  --datetime timestamp=2024-01-20T10:30:00Z \
  --float temperature=23.5 \
  --text unit=celsius \
  --tags sensors=room1,floor2

# Calendar event
agentroot metadata add #def456 \
  --text event_type=meeting \
  --datetime start=2024-01-25T14:00:00Z \
  --datetime end=2024-01-25T15:00:00Z \
  --tags attendees=alice,bob \
  --enum priority=high,medium,low

# Code file
agentroot metadata add #ghi789 \
  --text language=rust \
  --integer lines_of_code=150 \
  --tags frameworks=tokio,serde \
  --datetime last_modified=now \
  --text maintainer=alice@example.com

# SQL query result
agentroot metadata add #jkl012 \
  --text query_id=Q-2024-001 \
  --integer row_count=1500 \
  --float execution_time_ms=245.3 \
  --tags tables=users,orders \
  --datetime executed_at=now
```

## Schema Changes

### Documents Table (v5)

```sql
CREATE TABLE documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    collection TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT NOT NULL,
    hash TEXT NOT NULL REFERENCES content(hash),
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1,
    source_type TEXT NOT NULL DEFAULT 'file',
    source_uri TEXT,
    -- LLM-generated metadata
    llm_summary TEXT,
    llm_title TEXT,
    llm_keywords TEXT,
    llm_category TEXT,
    llm_intent TEXT,
    llm_concepts TEXT,
    llm_difficulty TEXT,
    llm_queries TEXT,
    llm_metadata_generated_at TEXT,
    llm_model TEXT,
    -- User-defined metadata (NEW in v5)
    user_metadata TEXT,  -- JSON blob of typed metadata
    UNIQUE(collection, path)
);

-- Index for faster metadata queries
CREATE INDEX IF NOT EXISTS idx_documents_user_metadata 
ON documents(user_metadata) WHERE user_metadata IS NOT NULL;
```

### Migration v4 → v5

```sql
ALTER TABLE documents ADD COLUMN user_metadata TEXT;
CREATE INDEX IF NOT EXISTS idx_documents_user_metadata 
ON documents(user_metadata) WHERE user_metadata IS NOT NULL;
UPDATE schema_version SET version = 5;
```

## Use Cases

### 1. Document Management System

```rust
// Add document metadata
db.add_metadata("#abc123", &MetadataBuilder::new()
    .text("author", "John Doe")
    .text("department", "Engineering")
    .datetime_now("published_date")
    .enum_value("status", "published", vec!["draft", "review", "published"])?
    .tags("topics", vec!["architecture", "design"])
    .build())?;

// Find all published engineering docs from 2024
let filter = MetadataFilter::And(vec![
    MetadataFilter::TextEq("department".into(), "Engineering".into()),
    MetadataFilter::EnumEq("status".into(), "published".into()),
    MetadataFilter::DateTimeAfter("published_date".into(), "2024-01-01T00:00:00Z".into()),
]);
```

### 2. Time-Series Monitoring

```rust
// Store sensor data point
db.add_metadata("#sensor001", &MetadataBuilder::new()
    .datetime_now("timestamp")
    .quantitative("temperature", 23.5, "celsius")
    .quantitative("humidity", 65.0, "percent")
    .tags("location", vec!["building-a", "floor-2", "room-101"])
    .text("sensor_id", "TEMP-001")
    .build())?;

// Query recent high-temperature readings
let filter = MetadataFilter::And(vec![
    MetadataFilter::DateTimeAfter("timestamp".into(), last_hour),
    MetadataFilter::FloatGt("temperature".into(), 25.0),
]);
```

### 3. Code Repository Analysis

```rust
// Tag code files
db.add_metadata("#code456", &MetadataBuilder::new()
    .text("language", "rust")
    .text("framework", "tokio")
    .integer("lines_of_code", 450)
    .integer("complexity", 12)
    .tags("features", vec!["async", "networking", "http"])
    .datetime_now("last_analyzed")
    .text("maintainer", "alice@example.com")
    .qualitative("code_quality", "high", vec!["low", "medium", "high"])?
    .build())?;

// Find complex async code needing review
let filter = MetadataFilter::And(vec![
    MetadataFilter::TagsContain("features".into(), "async".into()),
    MetadataFilter::IntegerGt("complexity".into(), 10),
]);
```

### 4. Data Lake Integration

```rust
// Index dataset metadata
db.add_metadata("#dataset789", &MetadataBuilder::new()
    .text("dataset_name", "customer_transactions")
    .text("schema_version", "v2.1")
    .integer("row_count", 1_000_000)
    .quantitative("size_mb", 450.5, "MB")
    .datetime(Utc.ymd(2024, 1, 20).and_hms(0, 0, 0))
    .tags("tables", vec!["transactions", "customers", "products"])
    .tags("regions", vec!["us-east", "eu-west"])
    .text("format", "parquet")
    .build())?;
```

## Next Steps

1. **Complete Schema Migration** (1-2 hours)
   - Update CREATE_TABLES
   - Add v4→v5 migration
   - Test migration on existing databases

2. **CLI Commands** (2-3 hours)
   - Implement metadata subcommands
   - Add parsers for typed values
   - Add formatted output

3. **Search Integration** (2-3 hours)
   - Add user_metadata to SearchResult
   - Combine with LLM metadata
   - Add metadata filters to search

4. **Testing** (1-2 hours)
   - Integration tests
   - CLI tests
   - Migration tests

5. **Documentation** (1-2 hours)
   - User guide
   - API examples
   - Migration guide

**Total Estimated Time**: 8-12 hours

## Questions to Resolve

1. **Metadata in FTS Index?**
   - Should user metadata be searchable via full-text?
   - Pros: Unified search
   - Cons: Type information lost, larger index

2. **Metadata Schema Validation?**
   - Should collections define metadata schemas?
   - Pros: Type safety, validation
   - Cons: Less flexibility

3. **Metadata Versioning?**
   - Track metadata changes over time?
   - Pros: Audit trail
   - Cons: Storage overhead

4. **Metadata Inheritance?**
   - Should collections have default metadata?
   - Should tags inherit from folders?

## Future Enhancements

1. **Metadata Schemas**
   - Define collection-level metadata schemas
   - Validation against schemas
   - Auto-complete in CLI

2. **Metadata Triggers**
   - Auto-update metadata on certain events
   - Computed metadata fields
   - Metadata pipelines

3. **Metadata Export/Import**
   - Export metadata to JSON/CSV
   - Bulk import from external sources
   - Sync with external systems

4. **Metadata Analytics**
   - Aggregate queries over metadata
   - Trends and patterns
   - Metadata quality metrics

5. **Graph Relationships**
   - Link documents via metadata
   - Query relationship graphs
   - Visualize connections
