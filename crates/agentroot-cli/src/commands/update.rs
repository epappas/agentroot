//! Update command

use crate::app::UpdateArgs;
use crate::progress::ProgressReporter;
use agentroot_core::Database;
use anyhow::Result;

pub async fn run(args: UpdateArgs, db: &Database) -> Result<()> {
    let collections = db.list_collections()?;

    if collections.is_empty() {
        println!("No collections to update");
        return Ok(());
    }

    let mut progress = ProgressReporter::new(collections.len());

    for coll in collections {
        progress.set_message(&format!("Updating {}", coll.name));

        if args.pull {
            let status = std::process::Command::new("git")
                .args(["pull"])
                .current_dir(&coll.path)
                .status();

            if let Err(e) = status {
                eprintln!("Warning: git pull failed for {}: {}", coll.name, e);
            }
        }

        let updated = db.reindex_collection(&coll.name)?;
        progress.increment();
        println!("{}: {} files updated", coll.name, updated);
    }

    progress.finish();
    Ok(())
}
