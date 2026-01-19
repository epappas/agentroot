//! Get document command

use anyhow::Result;
use agentroot_core::Database;
use crate::app::{GetArgs, MultiGetArgs, OutputFormat};

pub async fn run(args: GetArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let content = db.get_document(&args.file)?;

    let lines: Vec<&str> = content.lines().collect();
    let start = args.from.unwrap_or(1).saturating_sub(1);
    let end = args.l.map(|l| start + l).unwrap_or(lines.len());

    let selected: Vec<&str> = lines.iter()
        .skip(start)
        .take(end - start)
        .copied()
        .collect();

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "file": args.file,
                "content": selected.join("\n"),
                "start_line": start + 1,
                "line_count": selected.len()
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            for (i, line) in selected.iter().enumerate() {
                if args.line_numbers {
                    println!("{:>4} {}", start + i + 1, line);
                } else {
                    println!("{}", line);
                }
            }
        }
    }
    Ok(())
}

pub async fn run_multi(args: MultiGetArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let docs = db.get_documents_by_pattern(&args.pattern)?;

    match format {
        OutputFormat::Json => {
            let output: Vec<_> = docs.into_iter().map(|d| {
                serde_json::json!({
                    "path": d.path,
                    "content": d.content,
                })
            }).collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            for doc in docs {
                if doc.content.len() > args.max_bytes {
                    eprintln!("Skipping {} (exceeds max-bytes)", doc.path);
                    continue;
                }
                println!("--- {} ---", doc.path);
                let lines: Vec<&str> = doc.content.lines().collect();
                let limit = args.l.unwrap_or(lines.len());
                for (i, line) in lines.iter().take(limit).enumerate() {
                    if args.line_numbers {
                        println!("{:>4} {}", i + 1, line);
                    } else {
                        println!("{}", line);
                    }
                }
            }
        }
    }
    Ok(())
}
