//! Collection management commands

use crate::app::{CollectionAction, CollectionArgs};
use agentroot_core::Database;
use anyhow::Result;

pub async fn run(args: CollectionArgs, db: &Database) -> Result<()> {
    match args.action {
        CollectionAction::Add {
            path,
            name,
            mask,
            provider,
            config,
        } => {
            let collection_name = name.unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string()
            });

            let base_path = if provider == "file" {
                // For file provider, canonicalize the path
                path.canonicalize()?.to_string_lossy().to_string()
            } else {
                // For other providers (GitHub, URL, etc.), use path as-is
                path.to_string_lossy().to_string()
            };

            db.add_collection(
                &collection_name,
                &base_path,
                &mask,
                &provider,
                config.as_deref(),
            )?;

            println!(
                "Added collection '{}' (provider: {}, path: {})",
                collection_name, provider, base_path
            );
        }
        CollectionAction::List => {
            let collections = db.list_collections()?;
            if collections.is_empty() {
                println!("No collections");
            } else {
                for coll in collections {
                    println!(
                        "{}: {} ({}) [provider: {}, {} documents]",
                        coll.name, coll.path, coll.pattern, coll.provider_type, coll.document_count
                    );
                }
            }
        }
        CollectionAction::Remove { name } => {
            db.remove_collection(&name)?;
            println!("Removed collection '{}'", name);
        }
        CollectionAction::Rename { old_name, new_name } => {
            db.rename_collection(&old_name, &new_name)?;
            println!("Renamed collection '{}' to '{}'", old_name, new_name);
        }
    }
    Ok(())
}
