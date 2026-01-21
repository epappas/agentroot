//! Update command

use crate::app::UpdateArgs;
use crate::progress::ProgressReporter;
use agentroot_core::{Database, HttpMetadataGenerator, MetadataGenerator};
use anyhow::Result;
use std::sync::Arc;

pub async fn run(args: UpdateArgs, db: &Database, verbose: bool) -> Result<()> {
    let collections = db.list_collections()?;

    if collections.is_empty() {
        println!("No collections to update");
        return Ok(());
    }

    // Try to get HTTP metadata generator from environment
    let metadata_generator: Option<Arc<dyn MetadataGenerator>> =
        match HttpMetadataGenerator::from_env() {
            Ok(gen) => {
                if verbose {
                    println!("Using HTTP metadata service: {}", gen.model_name());
                }
                Some(Arc::new(gen))
            }
            Err(_) => {
                if verbose {
                    println!("No metadata service configured (set AGENTROOT_LLM_URL to enable)");
                }
                None
            }
        };

    let total_docs_before: usize = collections.iter().map(|c| c.document_count).sum();
    let mut progress = ProgressReporter::new(collections.len()).with_percentage(true);

    println!(
        "Updating {} collections ({} documents)...",
        collections.len(),
        total_docs_before
    );

    let mut total_updated = 0;
    let mut total_errors = 0;

    for coll in &collections {
        progress.set_message(&format!("Updating {} ({})", coll.name, coll.provider_type));

        if args.pull {
            if verbose {
                eprintln!("Running git pull in {}...", coll.path);
            }
            let status = std::process::Command::new("git")
                .args(["pull"])
                .current_dir(&coll.path)
                .status();

            if let Err(e) = status {
                eprintln!("Warning: git pull failed for {}: {}", coll.name, e);
                total_errors += 1;
            }
        }

        // Use reindex_collection_with_metadata to generate metadata if service is configured
        match db
            .reindex_collection_with_metadata(
                &coll.name,
                metadata_generator.as_ref().map(|g| g.as_ref()),
            )
            .await
        {
            Ok(updated) => {
                progress.increment();
                if updated > 0 || verbose {
                    println!("{}: {} files updated", coll.name, updated);
                }
                total_updated += updated;
            }
            Err(e) => {
                progress.increment();
                eprintln!("Error updating {}: {}", coll.name, e);
                total_errors += 1;
            }
        }
    }

    let collections_after = db.list_collections()?;
    let total_docs_after: usize = collections_after.iter().map(|c| c.document_count).sum();

    if total_errors > 0 {
        progress.finish_with_message(&format!("Completed with {} errors", total_errors));
    } else {
        progress.finish();
    }

    println!();
    println!(
        "Summary: {} files updated, {} total documents",
        total_updated, total_docs_after
    );

    if total_docs_after > total_docs_before {
        println!(
            "  {} new documents added",
            total_docs_after - total_docs_before
        );
    } else if total_docs_after < total_docs_before {
        println!(
            "  {} documents removed",
            total_docs_before - total_docs_after
        );
    }

    if total_errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
