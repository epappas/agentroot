//! Metadata command

use crate::app::{MetadataAction, MetadataArgs, OutputFormat};
use agentroot_core::{Database, LlamaMetadataGenerator};
use anyhow::Result;

pub async fn run(args: MetadataArgs, db: &Database, format: OutputFormat) -> Result<()> {
    match args.action {
        MetadataAction::Refresh {
            collection,
            all,
            doc,
            force,
            model,
        } => run_refresh(db, collection, all, doc, force, model).await,
        MetadataAction::Show { docid } => run_show(db, &docid, format).await,
    }
}

async fn run_refresh(
    db: &Database,
    collection: Option<String>,
    all: bool,
    doc: Option<String>,
    force: bool,
    model_path: Option<std::path::PathBuf>,
) -> Result<()> {
    let generator = if let Some(path) = model_path {
        LlamaMetadataGenerator::new(path)?
    } else {
        LlamaMetadataGenerator::from_default()?
    };

    if let Some(doc_path) = doc {
        println!("Refreshing metadata for document: {}", doc_path);
        anyhow::bail!("Single document metadata refresh not yet implemented");
    } else if all {
        println!("Refreshing metadata for all collections...");
        let collections = db.list_collections()?;
        for coll in collections {
            println!("Processing collection: {}", coll.name);
            let updated = db
                .reindex_collection_with_metadata(&coll.name, Some(&generator))
                .await?;
            println!("  Updated {} documents", updated);
        }
        println!("Done!");
    } else if let Some(coll_name) = collection {
        println!("Refreshing metadata for collection: {}", coll_name);
        let updated = db
            .reindex_collection_with_metadata(&coll_name, Some(&generator))
            .await?;
        println!("Updated {} documents", updated);
    } else {
        anyhow::bail!("Must specify --all, a collection name, or --doc <path>");
    }

    if force {
        println!("Note: --force flag not yet implemented (cache clearing)");
    }

    Ok(())
}

async fn run_show(db: &Database, docid: &str, format: OutputFormat) -> Result<()> {
    let doc = db
        .find_by_docid(docid)?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", docid))?;

    let metadata = fetch_metadata_for_hash(db, &doc.hash)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&metadata)?;
            println!("{}", json);
        }
        _ => {
            if let Some(meta) = metadata {
                println!("Document: {}", doc.display_path);
                println!("Title: {}", doc.title);
                println!();
                println!("=== LLM-Generated Metadata ===");
                println!();
                println!("Semantic Title: {}", meta.semantic_title);
                println!("Category: {}", meta.category);
                println!("Difficulty: {}", meta.difficulty);
                println!();
                println!("Summary:");
                println!("{}", meta.summary);
                println!();
                println!("Keywords: {}", meta.keywords.join(", "));
                println!();
                println!("Concepts: {}", meta.concepts.join(", "));
                println!();
                println!("Intent:");
                println!("{}", meta.intent);
                println!();
                println!("Suggested Queries:");
                for query in &meta.suggested_queries {
                    println!("  - {}", query);
                }
            } else {
                println!("No metadata available for this document.");
                println!(
                    "Run 'agentroot metadata refresh {}' to generate metadata.",
                    doc.collection_name
                );
            }
        }
    }

    Ok(())
}

fn fetch_metadata_for_hash(
    db: &Database,
    hash: &str,
) -> Result<Option<agentroot_core::DocumentMetadata>> {
    let cache_key = format!("metadata:v1:{}", hash);

    let cached = db.get_llm_cache_public(&cache_key)?;
    if let Some(json) = cached {
        let metadata = serde_json::from_str(&json)?;
        return Ok(Some(metadata));
    }

    Ok(None)
}
