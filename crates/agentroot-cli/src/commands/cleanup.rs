//! Cleanup command

use agentroot_core::Database;
use anyhow::Result;

pub async fn run(db: &Database) -> Result<()> {
    let orphaned = db.cleanup_orphaned_content()?;
    println!("Removed {} orphaned content entries", orphaned);

    let vectors = db.cleanup_orphaned_vectors()?;
    println!("Removed {} orphaned vectors", vectors);

    db.vacuum()?;
    println!("Database vacuumed");

    Ok(())
}
