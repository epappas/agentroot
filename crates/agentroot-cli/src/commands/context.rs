//! Context management commands

use anyhow::Result;
use agentroot_core::Database;
use crate::app::{ContextArgs, ContextAction};

pub async fn run(args: ContextArgs, db: &Database) -> Result<()> {
    match args.action {
        ContextAction::Add { path, context } => {
            db.add_context(&path, &context)?;
            println!("Added context for '{}'", path);
        }
        ContextAction::List => {
            let contexts = db.list_contexts()?;
            if contexts.is_empty() {
                println!("No contexts");
            } else {
                for ctx in contexts {
                    println!("{}: {}", ctx.path, ctx.context);
                }
            }
        }
        ContextAction::Check => {
            let missing = db.check_missing_contexts()?;
            if missing.is_empty() {
                println!("All collections have context");
            } else {
                println!("Missing context for:");
                for path in missing {
                    println!("  {}", path);
                }
            }
        }
        ContextAction::Remove { path } => {
            db.remove_context(&path)?;
            println!("Removed context for '{}'", path);
        }
    }
    Ok(())
}
