//! List command

use crate::app::{LsArgs, OutputFormat};
use agentroot_core::Database;
use anyhow::Result;

pub async fn run(args: LsArgs, db: &Database, format: OutputFormat) -> Result<()> {
    match args.path {
        None => {
            let collections = db.list_collections()?;
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&collections)?);
                }
                _ => {
                    for coll in collections {
                        println!("{}", coll.name);
                    }
                }
            }
        }
        Some(path) => {
            let docs = db.list_documents_by_prefix(&path)?;
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&docs)?);
                }
                OutputFormat::Files => {
                    for doc in docs {
                        println!("{}", doc.path);
                    }
                }
                _ => {
                    for doc in docs {
                        println!("{} #{}", doc.path, doc.docid);
                    }
                }
            }
        }
    }
    Ok(())
}
