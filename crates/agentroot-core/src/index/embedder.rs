//! Embedding pipeline with smart cache invalidation

use super::ast_chunker::{compute_chunk_hash, SemanticChunk, SemanticChunker};
use super::chunker::{chunk_by_chars, CHUNK_OVERLAP_CHARS, CHUNK_SIZE_CHARS};
use crate::db::{CacheLookupResult, Database};
use crate::error::Result;
use crate::llm::Embedder;
use std::path::Path;

const BATCH_SIZE: usize = 32;

/// Embedding progress
#[derive(Debug, Clone)]
pub struct EmbedProgress {
    pub total_docs: usize,
    pub processed_docs: usize,
    pub total_chunks: usize,
    pub processed_chunks: usize,
    pub cached_chunks: usize,
    pub computed_chunks: usize,
}

/// Embedding statistics
#[derive(Debug, Clone, Default)]
pub struct EmbedStats {
    pub total_documents: usize,
    pub embedded_documents: usize,
    pub total_chunks: usize,
    pub embedded_chunks: usize,
    pub cached_chunks: usize,
    pub computed_chunks: usize,
}

impl EmbedStats {
    pub fn cache_hit_rate(&self) -> f64 {
        if self.embedded_chunks == 0 {
            return 0.0;
        }
        self.cached_chunks as f64 / self.embedded_chunks as f64 * 100.0
    }
}

/// Chunk ready for embedding with cache metadata
struct ChunkToEmbed {
    seq: u32,
    text: String,
    position: usize,
    chunk_hash: String,
    cached_embedding: Option<Vec<f32>>,
}

/// Generate embeddings for documents with smart caching
pub async fn embed_documents(
    db: &Database,
    embedder: &dyn Embedder,
    model: &str,
    force: bool,
    progress: Option<Box<dyn Fn(EmbedProgress) + Send + Sync>>,
) -> Result<EmbedStats> {
    let docs = if force {
        db.get_all_content_with_paths()?
    } else {
        db.get_content_needing_embedding_with_paths()?
    };

    if docs.is_empty() {
        return Ok(EmbedStats::default());
    }

    let dimensions = embedder.dimensions();
    db.ensure_vec_table(dimensions)?;

    // Check model compatibility once upfront
    let cache_enabled = !force && db.check_model_compatibility(model, dimensions)?;
    db.register_model(model, dimensions)?;

    let total_docs = docs.len();
    let mut stats = EmbedStats {
        total_documents: total_docs,
        ..Default::default()
    };

    let chunker = SemanticChunker::new();

    for (doc_idx, (hash, content, path)) in docs.iter().enumerate() {
        let title = db.get_document_title_by_hash(hash)?;

        // Use semantic chunking if we have a file path
        let semantic_chunks = if let Some(p) = path {
            chunker.chunk(content, Path::new(p))?
        } else {
            fallback_to_semantic_chunks(content)
        };

        stats.total_chunks += semantic_chunks.len();

        // Prepare chunks with cache lookups
        let mut chunks_to_embed: Vec<ChunkToEmbed> = Vec::new();

        for (seq, chunk) in semantic_chunks.iter().enumerate() {
            let formatted_text = format_doc_for_embedding(&chunk.text, title.as_deref());

            // Try to find cached embedding (using fast lookup since we checked compatibility upfront)
            let cached = if cache_enabled {
                match db.get_cached_embedding_fast(&chunk.chunk_hash, model)? {
                    CacheLookupResult::Hit(emb) => Some(emb),
                    CacheLookupResult::Miss | CacheLookupResult::ModelMismatch => None,
                }
            } else {
                None
            };

            chunks_to_embed.push(ChunkToEmbed {
                seq: seq as u32,
                text: formatted_text,
                position: chunk.position,
                chunk_hash: chunk.chunk_hash.clone(),
                cached_embedding: cached,
            });
        }

        // Separate cached from needing computation
        let (cached, to_compute): (Vec<_>, Vec<_>) = chunks_to_embed
            .into_iter()
            .partition(|c| c.cached_embedding.is_some());

        // Store cached embeddings
        for chunk in cached {
            let embedding = chunk.cached_embedding.unwrap();
            db.insert_chunk_embedding(
                hash,
                chunk.seq,
                chunk.position,
                &chunk.chunk_hash,
                model,
                &embedding,
            )?;
            stats.embedded_chunks += 1;
            stats.cached_chunks += 1;
        }

        // Batch embed new chunks
        for batch in to_compute.chunks(BATCH_SIZE) {
            let texts: Vec<String> = batch.iter().map(|c| c.text.clone()).collect();
            let embeddings = embedder.embed_batch(&texts).await?;

            for (chunk, embedding) in batch.iter().zip(embeddings.iter()) {
                db.insert_chunk_embedding(
                    hash,
                    chunk.seq,
                    chunk.position,
                    &chunk.chunk_hash,
                    model,
                    embedding,
                )?;
                stats.embedded_chunks += 1;
                stats.computed_chunks += 1;
            }
        }

        stats.embedded_documents += 1;

        if let Some(ref cb) = progress {
            cb(EmbedProgress {
                total_docs,
                processed_docs: doc_idx + 1,
                total_chunks: stats.total_chunks,
                processed_chunks: stats.embedded_chunks,
                cached_chunks: stats.cached_chunks,
                computed_chunks: stats.computed_chunks,
            });
        }
    }

    Ok(stats)
}

/// Fallback: convert character-based chunks to semantic chunks with hashes
fn fallback_to_semantic_chunks(content: &str) -> Vec<SemanticChunk> {
    let char_chunks = chunk_by_chars(content, CHUNK_SIZE_CHARS, CHUNK_OVERLAP_CHARS);

    char_chunks
        .into_iter()
        .map(|c| {
            let chunk_hash = compute_chunk_hash(&c.text, "", "");
            SemanticChunk {
                text: c.text,
                chunk_type: super::ast_chunker::ChunkType::Text,
                chunk_hash,
                position: c.position,
                token_count: c.token_count,
                metadata: super::ast_chunker::ChunkMetadata::default(),
            }
        })
        .collect()
}

fn format_doc_for_embedding(text: &str, title: Option<&str>) -> String {
    format!("title: {} | text: {}", title.unwrap_or("none"), text)
}

impl Database {
    /// Get all content hashes and content
    pub fn get_all_content(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.doc FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1",
        )?;
        let results = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// Get all content with file paths
    pub fn get_all_content_with_paths(&self) -> Result<Vec<(String, String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.doc, d.path FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1
             GROUP BY c.hash",
        )?;
        let results = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// Get content needing embedding with file paths
    pub fn get_content_needing_embedding_with_paths(
        &self,
    ) -> Result<Vec<(String, String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.hash, c.doc, d.path FROM content c
             JOIN documents d ON d.hash = c.hash AND d.active = 1
             WHERE c.hash NOT IN (SELECT DISTINCT hash FROM content_vectors)
             GROUP BY c.hash",
        )?;
        let results = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// Get document title by hash
    pub fn get_document_title_by_hash(&self, hash: &str) -> Result<Option<String>> {
        let result = self.conn.query_row(
            "SELECT title FROM documents WHERE hash = ?1 AND active = 1 LIMIT 1",
            rusqlite::params![hash],
            |row| row.get(0),
        );
        match result {
            Ok(title) => Ok(Some(title)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
