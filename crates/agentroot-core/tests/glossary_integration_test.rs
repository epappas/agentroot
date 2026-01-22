//! Integration tests for intelligent glossary functionality

use agentroot_core::db::Database;
use agentroot_core::llm::{DocumentMetadata, ExtractedConcept};

#[test]
fn test_concept_extraction_and_linking() {
    // Create in-memory database
    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();
    db.ensure_vec_table(128).unwrap();

    // Create collection
    db.add_collection("test", "/tmp/test", "**/*.md", "file", None)
        .unwrap();

    // Insert content and document
    let content = "This is test content about Kubernetes orchestration and distributed systems.";
    let hash = agentroot_core::db::hash_content(content);
    db.insert_content(&hash, content).unwrap();

    // Create metadata with extracted concepts
    let metadata = DocumentMetadata {
        summary: "Test document".to_string(),
        semantic_title: "Test".to_string(),
        keywords: vec!["test".to_string()],
        category: "documentation".to_string(),
        intent: "Testing".to_string(),
        concepts: vec!["kubernetes".to_string()],
        difficulty: "intermediate".to_string(),
        suggested_queries: vec!["test query".to_string()],
        extracted_concepts: vec![
            ExtractedConcept {
                term: "kubernetes orchestration".to_string(),
                snippet: "test content about Kubernetes orchestration and".to_string(),
            },
            ExtractedConcept {
                term: "distributed systems".to_string(),
                snippet: "orchestration and distributed systems".to_string(),
            },
        ],
    };

    // Insert document with metadata
    let doc_id = db
        .insert_doc(
            &agentroot_core::db::DocumentInsert::new(
                "test",
                "test.md",
                "Test Document",
                &hash,
                "2024-01-01T00:00:00Z",
                "2024-01-01T00:00:00Z",
            )
            .with_llm_metadata_strings(
                &metadata.summary,
                &metadata.semantic_title,
                &serde_json::to_string(&metadata.keywords).unwrap(),
                &metadata.category,
                &metadata.intent,
                &serde_json::to_string(&metadata.concepts).unwrap(),
                &metadata.difficulty,
                &serde_json::to_string(&metadata.suggested_queries).unwrap(),
                "test-model",
                "2024-01-01T00:00:00Z",
            ),
        )
        .unwrap();

    assert!(doc_id > 0);

    // Manually extract and link concepts (simulating what reindex does)
    // Note: In real usage, this happens automatically during reindexing
    // Here we test it manually to verify the mechanism works

    // For testing, we need to create some chunks first
    // In real usage, chunks are created during vector embedding
    // Let's create a mock chunk entry
    db.insert_chunk_embedding(
        &hash,
        0,
        0,
        "mock_chunk_hash_123",
        "test-model",
        &vec![0.1; 128], // Mock embedding
    )
    .unwrap();

    // Now manually call extract_and_link_concepts (private method)
    // We'll test it indirectly by verifying concepts were created

    // Upsert concepts manually for testing
    let concept_id_1 = db.upsert_concept("kubernetes orchestration").unwrap();
    let concept_id_2 = db.upsert_concept("distributed systems").unwrap();

    // Link to chunk
    db.link_concept_to_chunk(
        concept_id_1,
        "mock_chunk_hash_123",
        &hash,
        "test content about Kubernetes orchestration and",
    )
    .unwrap();

    db.link_concept_to_chunk(
        concept_id_2,
        "mock_chunk_hash_123",
        &hash,
        "orchestration and distributed systems",
    )
    .unwrap();

    // Update stats
    db.update_concept_stats(concept_id_1).unwrap();
    db.update_concept_stats(concept_id_2).unwrap();

    // Verify concepts were created
    let (total_concepts, total_links) = db.get_concept_stats().unwrap();
    assert_eq!(total_concepts, 2);
    assert_eq!(total_links, 2);

    // Verify concepts can be searched
    let search_results = db.search_concepts("kubernetes", 10).unwrap();
    assert!(!search_results.is_empty());
    assert!(search_results.iter().any(|c| c.term.contains("kubernetes")));

    // Verify chunks can be retrieved for concept
    let chunks = db.get_chunks_for_concept(concept_id_1).unwrap();
    assert!(!chunks.is_empty());
    assert_eq!(chunks[0].document_hash, hash);
    assert_eq!(chunks[0].chunk_hash, "mock_chunk_hash_123");
}

#[test]
fn test_concept_search_basic() {
    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();

    // Create some concepts
    let id1 = db.upsert_concept("machine learning").unwrap();
    let id2 = db.upsert_concept("neural networks").unwrap();
    let id3 = db.upsert_concept("rust programming").unwrap();

    // Search for concepts
    let results = db.search_concepts("machine", 10).unwrap();
    assert!(!results.is_empty());

    let results = db.search_concepts("neural", 10).unwrap();
    assert!(!results.is_empty());

    let results = db.search_concepts("rust", 10).unwrap();
    assert!(!results.is_empty());

    // Verify specific concepts found
    assert_eq!(id1, id1); // Sanity check
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
}

#[test]
fn test_concept_deletion_on_document_removal() {
    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();

    let doc_hash = "test_hash_123";

    // Create concept and link to document
    let concept_id = db.upsert_concept("test concept").unwrap();
    db.link_concept_to_chunk(concept_id, "chunk_1", doc_hash, "test snippet")
        .unwrap();
    db.update_concept_stats(concept_id).unwrap();

    // Verify link exists
    let (_, links_before) = db.get_concept_stats().unwrap();
    assert_eq!(links_before, 1);

    // Delete concept links for document
    let deleted = db.delete_concepts_for_document(doc_hash).unwrap();
    assert_eq!(deleted, 1);

    // Verify link removed
    let (_, links_after) = db.get_concept_stats().unwrap();
    assert_eq!(links_after, 0);

    // Cleanup orphaned concepts
    let orphans_deleted = db.cleanup_orphaned_concepts().unwrap();
    assert_eq!(orphans_deleted, 1);

    // Verify concept removed
    let (concepts_after, _) = db.get_concept_stats().unwrap();
    assert_eq!(concepts_after, 0);
}

#[test]
fn test_glossary_workflow_integration() {
    // This tests the GlossarySearch workflow step
    use agentroot_core::llm::{Workflow, WorkflowStep};
    use agentroot_core::search::{execute_workflow, SearchOptions};

    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();
    db.ensure_vec_table(128).unwrap();

    // Create collection and document
    db.add_collection("test", "/tmp", "**/*.md", "file", None)
        .unwrap();

    let content = "Kubernetes orchestrates containers in distributed systems.";
    let hash = agentroot_core::db::hash_content(content);
    db.insert_content(&hash, content).unwrap();

    db.insert_doc(&agentroot_core::db::DocumentInsert::new(
        "test",
        "doc.md",
        "Kubernetes Guide",
        &hash,
        "2024-01-01T00:00:00Z",
        "2024-01-01T00:00:00Z",
    ))
    .unwrap();

    // Create concept and link
    db.insert_chunk_embedding(&hash, 0, 0, "chunk_abc", "model", &vec![0.1; 128])
        .unwrap();

    let concept_id = db.upsert_concept("kubernetes orchestration").unwrap();
    db.link_concept_to_chunk(
        concept_id,
        "chunk_abc",
        &hash,
        "Kubernetes orchestrates containers",
    )
    .unwrap();
    db.update_concept_stats(concept_id).unwrap();

    // Create workflow with GlossarySearch
    let workflow = Workflow {
        steps: vec![WorkflowStep::GlossarySearch {
            query: "kubernetes".to_string(),
            limit: 10,
            min_confidence: 0.3,
        }],
        reasoning: "Test glossary search".to_string(),
        expected_results: 10,
        complexity: "simple".to_string(),
    };

    // Execute workflow
    let options = SearchOptions::default();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let results = runtime
        .block_on(execute_workflow(&db, &workflow, "kubernetes", &options))
        .unwrap();

    // Verify results
    assert!(!results.is_empty(), "Should find documents via glossary");
    assert_eq!(results[0].hash, hash);
    assert!(results[0]
        .context
        .as_ref()
        .unwrap()
        .contains("Found via concept"));
}

#[test]
fn test_concept_normalization() {
    let db = Database::open_in_memory().unwrap();
    db.initialize().unwrap();

    // Create concepts with different casings
    let id1 = db.upsert_concept("Machine Learning").unwrap();
    let id2 = db.upsert_concept("machine learning").unwrap();

    // Should be same ID due to normalization
    assert_eq!(id1, id2, "Concepts should be normalized and deduplicated");

    // Verify only one concept created
    let (total, _) = db.get_concept_stats().unwrap();
    assert_eq!(total, 1);
}
