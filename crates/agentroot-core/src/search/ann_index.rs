//! HNSW approximate nearest neighbor index for vector search

use crate::db::vectors::cosine_similarity;
use crate::db::Database;
use crate::error::Result;
use instant_distance::{Builder, HnswMap, Search};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

/// Minimum embedding count to justify building an ANN index.
/// Below this threshold, brute-force is fast enough.
const ANN_THRESHOLD: usize = 1000;

/// Wrapper for f32 vectors implementing instant_distance::Point
#[derive(Clone)]
struct EmbeddingPoint {
    values: Vec<f32>,
}

impl instant_distance::Point for EmbeddingPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Cosine distance = 1.0 - cosine_similarity
        1.0 - cosine_similarity(&self.values, &other.values)
    }
}

/// HNSW-backed approximate nearest neighbor index
pub struct AnnIndex {
    index: RwLock<Option<HnswMap<EmbeddingPoint, String>>>,
    embedding_count: AtomicUsize,
}

impl AnnIndex {
    pub fn new() -> Self {
        Self {
            index: RwLock::new(None),
            embedding_count: AtomicUsize::new(0),
        }
    }

    /// Build index from database embeddings.
    /// Skips building if fewer than ANN_THRESHOLD embeddings.
    pub fn build_from_db(db: &Database, collection: Option<&str>) -> Result<Self> {
        let embeddings = match collection {
            Some(c) => db.get_embeddings_for_collection(c)?,
            None => db.get_all_embeddings()?,
        };

        let count = embeddings.len();
        let ann = Self::new();
        ann.embedding_count.store(count, Ordering::Relaxed);

        if count < ANN_THRESHOLD {
            tracing::debug!(
                "Skipping ANN index build: {} embeddings < {} threshold",
                count,
                ANN_THRESHOLD
            );
            return Ok(ann);
        }

        let (points, keys): (Vec<EmbeddingPoint>, Vec<String>) = embeddings
            .into_iter()
            .map(|(key, values)| (EmbeddingPoint { values }, key))
            .unzip();

        let hnsw_map = Builder::default().build(points, keys);

        *ann.index.write().map_err(|e| {
            crate::error::AgentRootError::Search(format!("ANN lock poisoned: {}", e))
        })? = Some(hnsw_map);

        tracing::info!("Built ANN index with {} embeddings", count);
        Ok(ann)
    }

    /// Search the ANN index for k nearest neighbors.
    /// Returns (hash_seq, cosine_similarity) pairs.
    /// Returns empty vec if index not built.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        let guard = match self.index.read() {
            Ok(g) => g,
            Err(_) => return vec![],
        };

        let map = match guard.as_ref() {
            Some(m) => m,
            None => return vec![],
        };

        let query_point = EmbeddingPoint {
            values: query.to_vec(),
        };
        let mut search = Search::default();

        map.search(&query_point, &mut search)
            .take(k)
            .map(|item| {
                let similarity = 1.0 - item.distance;
                (item.value.clone(), similarity)
            })
            .collect()
    }

    /// Whether the HNSW index has been built
    pub fn is_built(&self) -> bool {
        self.index.read().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Number of embeddings loaded (even if index wasn't built)
    pub fn len(&self) -> usize {
        self.embedding_count.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for AnnIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::hash_content;

    fn setup_db_with_embeddings(count: usize) -> (Database, tempfile::TempDir) {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("ann_test.db");
        let db = Database::open(&db_path).unwrap();
        db.initialize().unwrap();

        db.add_collection("test", "/tmp/test", "**/*.md", "file", None)
            .unwrap();
        db.ensure_vec_table(4).unwrap();

        for i in 0..count {
            let content = format!("document {}", i);
            let hash = hash_content(&content);
            db.insert_content(&hash, &content).unwrap();
            db.insert_document(
                "test",
                &format!("doc_{}.md", i),
                &format!("Doc {}", i),
                &hash,
                "2024-01-01",
                "2024-01-01",
                "file",
                None,
            )
            .unwrap();

            // Simple deterministic embedding
            let embedding = vec![
                (i as f32).sin(),
                (i as f32).cos(),
                (i as f32 * 0.5).sin(),
                (i as f32 * 0.5).cos(),
            ];
            db.insert_embedding(&hash, 0, 0, "test-model", &embedding)
                .unwrap();
        }

        (db, temp)
    }

    #[test]
    fn test_build_below_threshold() {
        let (db, _temp) = setup_db_with_embeddings(10);
        let ann = AnnIndex::build_from_db(&db, None).unwrap();

        assert!(!ann.is_built());
        assert_eq!(ann.len(), 10);

        // Search should return empty when not built
        let results = ann.search(&[0.5, 0.5, 0.5, 0.5], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_build_and_search() {
        // Build with enough embeddings to trigger index
        let (db, _temp) = setup_db_with_embeddings(ANN_THRESHOLD + 10);
        let ann = AnnIndex::build_from_db(&db, None).unwrap();

        assert!(ann.is_built());
        assert_eq!(ann.len(), ANN_THRESHOLD + 10);

        // Search returns results
        let results = ann.search(&[1.0, 0.0, 0.5, 0.5], 5);
        assert_eq!(results.len(), 5);

        // Results have similarities
        for (key, sim) in &results {
            assert!(!key.is_empty());
            assert!(*sim >= -1.0 && *sim <= 1.0);
        }
    }

    #[test]
    fn test_search_empty_index() {
        let ann = AnnIndex::new();
        let results = ann.search(&[1.0, 0.0], 5);
        assert!(results.is_empty());
        assert!(!ann.is_built());
        assert!(ann.is_empty());
    }
}
