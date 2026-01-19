//! Collection management commands

use crate::app::{CollectionAction, CollectionArgs};
use agentroot_core::Database;
use anyhow::Result;

pub async fn run(args: CollectionArgs, db: &Database) -> Result<()> {
    match args.action {
        CollectionAction::Add { path, name, mask } => {
            let collection_name = name.unwrap_or_else(|| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string()
            });
            let abs_path = path.canonicalize()?;
            db.add_collection(
                &collection_name,
                abs_path.to_str().unwrap(),
                &mask,
                "file",
                None,
            )?;
            println!(
                "Added collection '{}' at {}",
                collection_name,
                abs_path.display()
            );
        }
        CollectionAction::List => {
            let collections = db.list_collections()?;
            if collections.is_empty() {
                println!("No collections");
            } else {
                for coll in collections {
                    println!("{}: {} ({})", coll.name, coll.path, coll.pattern);
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
