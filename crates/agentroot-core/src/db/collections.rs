//! Collection operations

use super::Database;
use crate::error::Result;
use chrono::Utc;
use rusqlite::params;

/// Collection info
#[derive(Debug, Clone, serde::Serialize)]
pub struct CollectionInfo {
    pub name: String,
    pub path: String,
    pub pattern: String,
    pub document_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub provider_type: String,
    pub provider_config: Option<String>,
}

impl Database {
    /// Add a new collection
    pub fn add_collection(
        &self,
        name: &str,
        path: &str,
        pattern: &str,
        provider_type: &str,
        provider_config: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO collections (name, path, pattern, created_at, updated_at, provider_type, provider_config)
             VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6)",
            params![name, path, pattern, now, provider_type, provider_config],
        )?;
        Ok(())
    }

    /// Remove a collection and its documents
    pub fn remove_collection(&self, name: &str) -> Result<bool> {
        // Deactivate all documents
        self.conn.execute(
            "UPDATE documents SET active = 0 WHERE collection = ?1",
            params![name],
        )?;

        // Remove collection
        let rows = self
            .conn
            .execute("DELETE FROM collections WHERE name = ?1", params![name])?;

        Ok(rows > 0)
    }

    /// Rename a collection
    pub fn rename_collection(&self, old_name: &str, new_name: &str) -> Result<bool> {
        let now = Utc::now().to_rfc3339();

        // Update documents
        self.conn.execute(
            "UPDATE documents SET collection = ?2 WHERE collection = ?1",
            params![old_name, new_name],
        )?;

        // Update collection
        let rows = self.conn.execute(
            "UPDATE collections SET name = ?2, updated_at = ?3 WHERE name = ?1",
            params![old_name, new_name, now],
        )?;

        Ok(rows > 0)
    }

    /// List all collections with document counts
    pub fn list_collections(&self) -> Result<Vec<CollectionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name, c.path, c.pattern, c.created_at, c.updated_at,
                    (SELECT COUNT(*) FROM documents d WHERE d.collection = c.name AND d.active = 1),
                    c.provider_type, c.provider_config
             FROM collections c
             ORDER BY c.name",
        )?;

        let results = stmt
            .query_map([], |row| {
                Ok(CollectionInfo {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    pattern: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    document_count: row.get::<_, i64>(5)? as usize,
                    provider_type: row.get(6)?,
                    provider_config: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get collection by name
    pub fn get_collection(&self, name: &str) -> Result<Option<CollectionInfo>> {
        let result = self.conn.query_row(
            "SELECT c.name, c.path, c.pattern, c.created_at, c.updated_at,
                    (SELECT COUNT(*) FROM documents d WHERE d.collection = c.name AND d.active = 1),
                    c.provider_type, c.provider_config
             FROM collections c WHERE c.name = ?1",
            params![name],
            |row| {
                Ok(CollectionInfo {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    pattern: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    document_count: row.get::<_, i64>(5)? as usize,
                    provider_type: row.get(6)?,
                    provider_config: row.get(7)?,
                })
            },
        );
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update collection's updated_at timestamp
    pub fn touch_collection(&self, name: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE collections SET updated_at = ?2 WHERE name = ?1",
            params![name, now],
        )?;
        Ok(())
    }

    /// Reindex a collection using the provider system
    pub async fn reindex_collection(&self, name: &str) -> Result<usize> {
        let coll = self
            .get_collection(name)?
            .ok_or_else(|| crate::error::AgentRootError::CollectionNotFound(name.to_string()))?;

        let registry = crate::providers::ProviderRegistry::with_defaults();
        let provider = registry.get(&coll.provider_type).ok_or_else(|| {
            crate::error::AgentRootError::InvalidInput(format!(
                "Unknown provider type: {}",
                coll.provider_type
            ))
        })?;

        let mut config =
            crate::providers::ProviderConfig::new(coll.path.clone(), coll.pattern.clone());

        if let Some(provider_config) = &coll.provider_config {
            if let Ok(config_map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(provider_config)
            {
                for (key, value) in config_map {
                    config = config.with_option(key, value);
                }
            }
        }

        let items = provider.list_items(&config).await?;
        let mut updated = 0;

        for item in items {
            let now = Utc::now().to_rfc3339();

            if let Some(existing) = self.find_active_document(name, &item.uri)? {
                if existing.hash != item.hash {
                    self.insert_content(&item.hash, &item.content)?;
                    self.update_document(existing.id, &item.title, &item.hash, &now)?;
                    updated += 1;
                }
            } else {
                self.insert_content(&item.hash, &item.content)?;
                self.insert_document(
                    name,
                    &item.uri,
                    &item.title,
                    &item.hash,
                    &now,
                    &now,
                    &item.source_type,
                    item.metadata.get("source_uri").map(|s| s.as_str()),
                )?;
                updated += 1;
            }
        }

        self.touch_collection(name)?;
        Ok(updated)
    }

    /// Generate or fetch metadata from cache
    pub async fn generate_or_fetch_metadata(
        &self,
        content_hash: &str,
        content: &str,
        context: crate::llm::MetadataContext,
        generator: Option<&dyn crate::llm::MetadataGenerator>,
    ) -> Result<Option<crate::llm::DocumentMetadata>> {
        if generator.is_none() {
            return Ok(None);
        }

        let cache_key = format!("metadata:v1:{}", content_hash);

        if let Some(cached) = self.get_llm_cache(&cache_key)? {
            if let Ok(metadata) = serde_json::from_str::<crate::llm::DocumentMetadata>(&cached) {
                return Ok(Some(metadata));
            }
        }

        let gen = generator.unwrap();
        match gen.generate_metadata(content, &context).await {
            Ok(metadata) => {
                let cache_value = serde_json::to_string(&metadata)?;
                self.set_llm_cache(&cache_key, &cache_value, gen.model_name())?;
                Ok(Some(metadata))
            }
            Err(e) => {
                eprintln!("Metadata generation failed: {}. Skipping metadata.", e);
                Ok(None)
            }
        }
    }

    /// Get metadata from LLM cache (public API)
    pub fn get_llm_cache_public(&self, key: &str) -> Result<Option<String>> {
        self.get_llm_cache(key)
    }

    /// Get metadata from LLM cache
    fn get_llm_cache(&self, key: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT value FROM llm_cache WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set metadata in LLM cache
    fn set_llm_cache(&self, key: &str, value: &str, model: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO llm_cache (key, value, model, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![key, value, model, now],
        )?;
        Ok(())
    }

    /// Build metadata context from source item
    fn build_metadata_context(
        &self,
        item: &crate::providers::SourceItem,
        collection_name: &str,
        coll: &CollectionInfo,
    ) -> crate::llm::MetadataContext {
        let path = std::path::Path::new(&item.uri);
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());

        crate::llm::MetadataContext::new(item.source_type.clone(), collection_name.to_string())
            .with_extension(extension.unwrap_or_default())
            .with_provider_config(coll.provider_config.clone().unwrap_or_default())
    }

    /// Insert document with metadata
    fn insert_document_with_metadata(
        &self,
        collection: &str,
        path: &str,
        title: &str,
        hash: &str,
        created_at: &str,
        modified_at: &str,
        source_type: &str,
        source_uri: Option<&str>,
        metadata: &crate::llm::DocumentMetadata,
        model_name: &str,
    ) -> Result<i64> {
        let keywords_json = serde_json::to_string(&metadata.keywords)?;
        let concepts_json = serde_json::to_string(&metadata.concepts)?;
        let queries_json = serde_json::to_string(&metadata.suggested_queries)?;
        let now = Utc::now().to_rfc3339();

        let doc = super::documents::DocumentInsert::new(
            collection,
            path,
            title,
            hash,
            created_at,
            modified_at,
        )
        .with_source_type(source_type)
        .with_source_uri(source_uri.unwrap_or(""))
        .with_llm_metadata_strings(
            &metadata.summary,
            &metadata.semantic_title,
            &keywords_json,
            &metadata.category,
            &metadata.intent,
            &concepts_json,
            &metadata.difficulty,
            &queries_json,
            model_name,
            &now,
        );

        self.insert_doc(&doc)
    }

    /// Update document with metadata
    fn update_document_with_metadata(
        &self,
        id: i64,
        title: &str,
        hash: &str,
        modified_at: &str,
        metadata: &crate::llm::DocumentMetadata,
        model_name: &str,
    ) -> Result<()> {
        let keywords_json = serde_json::to_string(&metadata.keywords)?;
        let concepts_json = serde_json::to_string(&metadata.concepts)?;
        let queries_json = serde_json::to_string(&metadata.suggested_queries)?;
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE documents 
             SET title = ?2, hash = ?3, modified_at = ?4,
                 llm_summary = ?5, llm_title = ?6, llm_keywords = ?7, llm_category = ?8,
                 llm_intent = ?9, llm_concepts = ?10, llm_difficulty = ?11, llm_queries = ?12,
                 llm_metadata_generated_at = ?13, llm_model = ?14
             WHERE id = ?1",
            params![
                id,
                title,
                hash,
                modified_at,
                metadata.summary,
                metadata.semantic_title,
                keywords_json,
                metadata.category,
                metadata.intent,
                concepts_json,
                metadata.difficulty,
                queries_json,
                now,
                model_name
            ],
        )?;
        Ok(())
    }

    /// Extract concepts from metadata and link to document chunks
    fn extract_and_link_concepts(
        &self,
        doc_hash: &str,
        metadata: &crate::llm::DocumentMetadata,
    ) -> Result<()> {
        // Delete old concept links for this document (in case of re-indexing)
        self.delete_concepts_for_document(doc_hash)?;

        // Skip if no extracted concepts
        if metadata.extracted_concepts.is_empty() {
            return Ok(());
        }

        // Get chunk hashes for this document
        let chunk_hashes = self.get_chunk_hashes_for_doc(doc_hash)?;

        // If no chunks, we can't link concepts
        if chunk_hashes.is_empty() {
            tracing::debug!("No chunks found for document {}, skipping concept linking", doc_hash);
            return Ok(());
        }

        // For each extracted concept
        for extracted in &metadata.extracted_concepts {
            // Upsert concept (get or create)
            let concept_id = self.upsert_concept(&extracted.term)?;

            // Link concept to all chunks of this document
            // (Concepts are document-level, not chunk-specific)
            for (_, chunk_hash) in &chunk_hashes {
                self.link_concept_to_chunk(
                    concept_id,
                    chunk_hash,
                    doc_hash,
                    &extracted.snippet,
                )?;
            }

            // Update concept statistics
            self.update_concept_stats(concept_id)?;

            tracing::debug!(
                "Linked concept '{}' to {} chunks for document {}",
                extracted.term,
                chunk_hashes.len(),
                doc_hash
            );
        }

        Ok(())
    }

    /// Process and store chunks with LLM-generated metadata
    async fn process_chunks_with_metadata(
        &self,
        doc_hash: &str,
        content: &str,
        path: &str,
        chunk_generator: Option<&dyn crate::llm::LLMClient>,
    ) -> Result<usize> {
        use crate::index::ast_chunker::{language::Language, SemanticChunker};
        use crate::llm::{generate_batch_chunk_metadata, ChunkMetadata};
        use std::path::Path;

        // Delete old chunks for this document (in case of re-indexing)
        self.delete_chunks_for_document(doc_hash)?;

        // Create semantic chunks
        let chunker = SemanticChunker::new();
        let semantic_chunks = chunker.chunk(content, Path::new(path))?;

        if semantic_chunks.is_empty() {
            tracing::debug!("No chunks created for document {}", doc_hash);
            return Ok(0);
        }

        let now = Utc::now().to_rfc3339();
        let mut chunks_inserted = 0;

        // Generate metadata for all chunks if LLM client is provided
        let metadata_list: Option<Vec<ChunkMetadata>> = if let Some(client) = chunk_generator {
            let language = Language::from_path(Path::new(path)).map(|l| l.as_str());

            match generate_batch_chunk_metadata(
                &semantic_chunks,
                path,
                language,
                client,
            )
            .await
            {
                Ok(meta) => Some(meta),
                Err(e) => {
                    tracing::warn!("Failed to generate chunk metadata for {}: {}", path, e);
                    None
                }
            }
        } else {
            None
        };

        // Insert chunks with or without metadata
        for (seq, chunk) in semantic_chunks.iter().enumerate() {
            let chunk_hash = chunk.chunk_hash.clone();

            // Get metadata for this chunk if available
            let chunk_meta = metadata_list.as_ref().and_then(|list: &Vec<ChunkMetadata>| list.get(seq));

            // Extract metadata fields
            let (summary, purpose, concepts, labels, model_name) = if let Some(meta) = chunk_meta {
                (
                    Some(meta.summary.as_str()),
                    Some(meta.purpose.as_str()),
                    &meta.concepts,
                    &meta.labels,
                    Some("chunk-metadata"),
                )
            } else {
                (None, None, &vec![], &std::collections::HashMap::new(), None)
            };

            // Insert chunk
            self.insert_chunk(
                &chunk_hash,
                doc_hash,
                seq as i32,
                chunk.position as i32,
                &chunk.text,
                Some(&format!("{:?}", chunk.chunk_type)),
                chunk.metadata.breadcrumb.as_deref(),
                chunk.metadata.start_line as i32,
                chunk.metadata.end_line as i32,
                chunk.metadata.language,
                summary,
                purpose,
                concepts,
                labels,
                &vec![], // related_to - can be populated later via semantic analysis
                model_name,
                if chunk_meta.is_some() { Some(&now) } else { None },
                &now,
            )?;

            chunks_inserted += 1;
        }

        tracing::debug!(
            "Inserted {} chunks for document {} (with_metadata: {})",
            chunks_inserted,
            doc_hash,
            metadata_list.is_some()
        );

        Ok(chunks_inserted)
    }

    /// Reindex all documents in a collection with optional metadata generation
    pub async fn reindex_collection_with_metadata(
        &self,
        name: &str,
        generator: Option<&dyn crate::llm::MetadataGenerator>,
    ) -> Result<usize> {
        let coll = self
            .get_collection(name)?
            .ok_or_else(|| crate::error::AgentRootError::CollectionNotFound(name.to_string()))?;

        let registry = crate::providers::ProviderRegistry::with_defaults();
        let provider = registry.get(&coll.provider_type).ok_or_else(|| {
            crate::error::AgentRootError::InvalidInput(format!(
                "Unknown provider type: {}",
                coll.provider_type
            ))
        })?;

        let mut config =
            crate::providers::ProviderConfig::new(coll.path.clone(), coll.pattern.clone());

        if let Some(provider_config) = &coll.provider_config {
            if let Ok(config_map) =
                serde_json::from_str::<std::collections::HashMap<String, String>>(provider_config)
            {
                for (key, value) in config_map {
                    config = config.with_option(key, value);
                }
            }
        }

        let items = provider.list_items(&config).await?;
        let mut updated = 0;

        for item in items {
            let now = Utc::now().to_rfc3339();

            if let Some(existing) = self.find_active_document(name, &item.uri)? {
                let content_changed = existing.hash != item.hash;
                let needs_metadata = existing.llm_model.is_none() && generator.is_some();

                if content_changed || needs_metadata {
                    if content_changed {
                        self.insert_content(&item.hash, &item.content)?;
                    }

                    let metadata_opt = if generator.is_some() {
                        let context = self.build_metadata_context(&item, name, &coll);
                        self.generate_or_fetch_metadata(
                            &item.hash,
                            &item.content,
                            context,
                            generator,
                        )
                        .await?
                    } else {
                        None
                    };

                    if let Some(metadata) = metadata_opt {
                        self.update_document_with_metadata(
                            existing.id,
                            &item.title,
                            &item.hash,
                            &now,
                            &metadata,
                            generator.unwrap().model_name(),
                        )?;

                        // Process chunks with LLM metadata
                        let llm_client = generator.and_then(|g| g.llm_client());
                        self.process_chunks_with_metadata(&item.hash, &item.content, &item.uri, llm_client)
                            .await?;

                        // Extract and link concepts to chunks
                        self.extract_and_link_concepts(&item.hash, &metadata)?;
                    } else {
                        self.update_document(existing.id, &item.title, &item.hash, &now)?;

                        // Still create chunks without LLM metadata
                        self.process_chunks_with_metadata(&item.hash, &item.content, &item.uri, None)
                            .await?;
                    }
                    updated += 1;
                }
            } else {
                self.insert_content(&item.hash, &item.content)?;

                let metadata_opt = if generator.is_some() {
                    let context = self.build_metadata_context(&item, name, &coll);
                    self.generate_or_fetch_metadata(&item.hash, &item.content, context, generator)
                        .await?
                } else {
                    None
                };

                if let Some(metadata) = metadata_opt {
                    self.insert_document_with_metadata(
                        name,
                        &item.uri,
                        &item.title,
                        &item.hash,
                        &now,
                        &now,
                        &item.source_type,
                        item.metadata.get("source_uri").map(|s| s.as_str()),
                        &metadata,
                        generator.unwrap().model_name(),
                    )?;

                    // Process chunks with LLM metadata
                    let llm_client = generator.and_then(|g| g.llm_client());
                    self.process_chunks_with_metadata(&item.hash, &item.content, &item.uri, llm_client)
                        .await?;

                    // Extract and link concepts to chunks
                    self.extract_and_link_concepts(&item.hash, &metadata)?;
                } else {
                    self.insert_document(
                        name,
                        &item.uri,
                        &item.title,
                        &item.hash,
                        &now,
                        &now,
                        &item.source_type,
                        item.metadata.get("source_uri").map(|s| s.as_str()),
                    )?;

                    // Still create chunks without LLM metadata
                    self.process_chunks_with_metadata(&item.hash, &item.content, &item.uri, None)
                        .await?;
                }
                updated += 1;
            }
        }

        self.touch_collection(name)?;
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_stores_provider_info_correctly() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection(
            "test_file",
            "/tmp/test",
            "**/*.md",
            "file",
            Some(r#"{"exclude_hidden":"false"}"#),
        )
        .unwrap();

        db.add_collection(
            "test_github",
            "https://github.com/test/repo",
            "**/*.md",
            "github",
            None,
        )
        .unwrap();

        let provider_type_file: String = db
            .conn
            .query_row(
                "SELECT provider_type FROM collections WHERE name = 'test_file'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(provider_type_file, "file");

        let provider_config_file: Option<String> = db
            .conn
            .query_row(
                "SELECT provider_config FROM collections WHERE name = 'test_file'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            provider_config_file,
            Some(r#"{"exclude_hidden":"false"}"#.to_string())
        );

        let provider_type_github: String = db
            .conn
            .query_row(
                "SELECT provider_type FROM collections WHERE name = 'test_github'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(provider_type_github, "github");

        let provider_config_github: Option<String> = db
            .conn
            .query_row(
                "SELECT provider_config FROM collections WHERE name = 'test_github'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(provider_config_github, None);

        let collections = db.list_collections().unwrap();
        assert_eq!(collections.len(), 2);

        let file_coll = collections.iter().find(|c| c.name == "test_file").unwrap();
        assert_eq!(file_coll.provider_type, "file");
        assert_eq!(
            file_coll.provider_config.as_deref(),
            Some(r#"{"exclude_hidden":"false"}"#)
        );

        let github_coll = collections
            .iter()
            .find(|c| c.name == "test_github")
            .unwrap();
        assert_eq!(github_coll.provider_type, "github");
        assert_eq!(github_coll.provider_config, None);
    }

    #[test]
    fn test_documents_store_source_metadata() {
        use crate::db::hash_content;
        use chrono::Utc;

        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection("test", "/tmp", "**/*.md", "file", None)
            .unwrap();

        let content = "# Test Document";
        let hash = hash_content(content);
        db.insert_content(&hash, content).unwrap();

        let now = Utc::now().to_rfc3339();
        let doc_id = db
            .insert_document(
                "test",
                "doc1.md",
                "Test Document",
                &hash,
                &now,
                &now,
                "file",
                Some("/tmp/doc1.md"),
            )
            .unwrap();

        assert!(doc_id > 0);

        let source_type: String = db
            .conn
            .query_row(
                "SELECT source_type FROM documents WHERE id = ?1",
                [doc_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_type, "file");

        let source_uri: Option<String> = db
            .conn
            .query_row(
                "SELECT source_uri FROM documents WHERE id = ?1",
                [doc_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_uri, Some("/tmp/doc1.md".to_string()));

        db.insert_content(&hash, content).unwrap();
        let doc_id2 = db
            .insert_document(
                "test",
                "doc2.md",
                "Test Document 2",
                &hash,
                &now,
                &now,
                "github",
                Some("https://github.com/test/repo/doc2.md"),
            )
            .unwrap();

        let source_type2: String = db
            .conn
            .query_row(
                "SELECT source_type FROM documents WHERE id = ?1",
                [doc_id2],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_type2, "github");

        let source_uri2: Option<String> = db
            .conn
            .query_row(
                "SELECT source_uri FROM documents WHERE id = ?1",
                [doc_id2],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            source_uri2,
            Some("https://github.com/test/repo/doc2.md".to_string())
        );
    }

    #[tokio::test]
    async fn test_reindex_collection_uses_provider_system() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let base = temp.path();

        fs::write(base.join("doc1.md"), "# Document 1\nInitial content").unwrap();
        fs::write(base.join("doc2.md"), "# Document 2\nInitial content").unwrap();

        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection(
            "test",
            &base.to_string_lossy(),
            "**/*.md",
            "file",
            Some(r#"{"exclude_hidden":"false"}"#),
        )
        .unwrap();

        let updated = db.reindex_collection("test").await.unwrap();
        assert_eq!(updated, 2, "Should index 2 files on first run");

        let collections = db.list_collections().unwrap();
        assert_eq!(collections[0].document_count, 2);

        let doc_count: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM documents WHERE collection = 'test' AND active = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(doc_count, 2);

        let mut stmt = db
            .conn
            .prepare(
                "SELECT path, source_type FROM documents WHERE collection = 'test' ORDER BY path",
            )
            .unwrap();
        let sources: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].0, "doc1.md");
        assert_eq!(sources[0].1, "file");
        assert_eq!(sources[1].0, "doc2.md");
        assert_eq!(sources[1].1, "file");

        fs::write(base.join("doc1.md"), "# Document 1\nUpdated content").unwrap();

        let updated2 = db.reindex_collection("test").await.unwrap();
        assert_eq!(updated2, 1, "Should update only changed file");

        let collections2 = db.list_collections().unwrap();
        assert_eq!(
            collections2[0].document_count, 2,
            "Should still have 2 documents"
        );

        fs::write(base.join("doc3.md"), "# Document 3\nNew content").unwrap();

        let updated3 = db.reindex_collection("test").await.unwrap();
        assert_eq!(updated3, 1, "Should add new file");

        let collections3 = db.list_collections().unwrap();
        assert_eq!(
            collections3[0].document_count, 3,
            "Should now have 3 documents"
        );
    }

    #[tokio::test]
    async fn test_reindex_invalid_provider_type() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection("test", "/tmp", "**/*.md", "nonexistent_provider", None)
            .unwrap();

        let result = db.reindex_collection("test").await;
        assert!(result.is_err(), "Should error on invalid provider type");

        match result {
            Err(crate::error::AgentRootError::InvalidInput(msg)) => {
                assert!(msg.contains("Unknown provider type"));
                assert!(msg.contains("nonexistent_provider"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[tokio::test]
    async fn test_reindex_nonexistent_collection() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        let result = db.reindex_collection("nonexistent").await;
        assert!(result.is_err(), "Should error on nonexistent collection");

        match result {
            Err(crate::error::AgentRootError::CollectionNotFound(name)) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("Expected CollectionNotFound error"),
        }
    }

    #[test]
    fn test_add_collection_duplicate_name() {
        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection("test", "/tmp1", "**/*.md", "file", None)
            .unwrap();

        let result = db.add_collection("test", "/tmp2", "**/*.md", "file", None);
        assert!(result.is_err(), "Should error on duplicate collection name");
    }

    #[tokio::test]
    async fn test_reindex_with_malformed_provider_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let base = temp.path();
        fs::write(base.join("test.md"), "# Test").unwrap();

        let db = Database::open_in_memory().unwrap();
        db.initialize().unwrap();

        db.add_collection(
            "test",
            &base.to_string_lossy(),
            "**/*.md",
            "file",
            Some("malformed json that won't parse"),
        )
        .unwrap();

        let result = db.reindex_collection("test").await;
        assert!(
            result.is_ok(),
            "Should succeed despite malformed JSON config (uses defaults)"
        );
    }
}
