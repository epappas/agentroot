//! Metadata command

use crate::app::{MetadataAction, MetadataArgs, OutputFormat};
use agentroot_core::{
    Database, HttpMetadataGenerator, MetadataBuilder, MetadataFilter, MetadataGenerator,
    MetadataValue,
};
use anyhow::Result;
use std::sync::Arc;

pub async fn run(args: MetadataArgs, db: &Database, format: OutputFormat) -> Result<()> {
    match args.action {
        MetadataAction::Refresh {
            collection,
            all,
            doc,
            force,
        } => run_refresh(db, collection, all, doc, force).await,
        MetadataAction::Show { docid } => run_show(db, &docid, format).await,
        MetadataAction::Add {
            docid,
            text,
            integer,
            float,
            boolean,
            datetime,
            tags,
            enum_value,
            qualitative,
            quantitative,
        } => run_add(
            db,
            &docid,
            text,
            integer,
            float,
            boolean,
            datetime,
            tags,
            enum_value,
            qualitative,
            quantitative,
        ),
        MetadataAction::Get { docid } => run_get(db, &docid, format),
        MetadataAction::Remove { docid, fields } => run_remove(db, &docid, fields),
        MetadataAction::Clear { docid } => run_clear(db, &docid),
        MetadataAction::Query { filter, limit } => run_query(db, &filter, limit, format),
    }
}

async fn run_refresh(
    db: &Database,
    collection: Option<String>,
    all: bool,
    doc: Option<String>,
    force: bool,
) -> Result<()> {
    // Get HTTP metadata generator from environment variables
    let generator: Arc<dyn MetadataGenerator> = match HttpMetadataGenerator::from_env() {
        Ok(http_gen) => {
            println!("Using HTTP metadata service: {}", http_gen.model_name());
            Arc::new(http_gen)
        }
        Err(_) => {
            eprintln!("Error: No metadata generation service configured");
            eprintln!();
            eprintln!("AgentRoot requires an external LLM service for metadata generation.");
            eprintln!("Configure one by setting environment variables:");
            eprintln!();
            eprintln!("  export AGENTROOT_LLM_URL=\"https://your-service.com/v1\"");
            eprintln!("  export AGENTROOT_LLM_MODEL=\"Qwen/Qwen2.5-7B-Instruct\"");
            eprintln!();
            eprintln!("Supported services:");
            eprintln!("  - vLLM (https://docs.vllm.ai)");
            eprintln!("  - Basilica (https://basilica.ai) - Recommended");
            eprintln!("  - OpenAI (https://openai.com/api)");
            eprintln!("  - Any OpenAI-compatible API");
            eprintln!();
            eprintln!("See VLLM_SETUP.md for detailed instructions.");
            return Err(anyhow::anyhow!("No metadata generation service configured"));
        }
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
                .reindex_collection_with_metadata(&coll.name, Some(generator.as_ref()))
                .await?;
            println!("  Updated {} documents", updated);
        }
        println!("Done!");
    } else if let Some(coll_name) = collection {
        println!("Refreshing metadata for collection: {}", coll_name);
        let updated = db
            .reindex_collection_with_metadata(&coll_name, Some(generator.as_ref()))
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

fn run_add(
    db: &Database,
    docid: &str,
    text: Vec<String>,
    integer: Vec<String>,
    float: Vec<String>,
    boolean: Vec<String>,
    datetime: Vec<String>,
    tags: Vec<String>,
    enum_value: Vec<String>,
    qualitative: Vec<String>,
    quantitative: Vec<String>,
) -> Result<()> {
    let mut builder = MetadataBuilder::new();

    for item in text {
        let (key, value) = parse_key_value(&item)?;
        builder = builder.text(&key, value);
    }

    for item in integer {
        let (key, value) = parse_key_value(&item)?;
        let num: i64 = value.parse()?;
        builder = builder.integer(&key, num);
    }

    for item in float {
        let (key, value) = parse_key_value(&item)?;
        let num: f64 = value.parse()?;
        builder = builder.float(&key, num);
    }

    for item in boolean {
        let (key, value) = parse_key_value(&item)?;
        let bool_val = value
            .parse::<bool>()
            .or_else(|_| match value.to_lowercase().as_str() {
                "yes" | "y" | "1" => Ok(true),
                "no" | "n" | "0" => Ok(false),
                _ => Err(anyhow::anyhow!("Invalid boolean value: {}", value)),
            })?;
        builder = builder.boolean(&key, bool_val);
    }

    for item in datetime {
        let (key, value) = parse_key_value(&item)?;
        let dt = if value == "now" {
            chrono::Utc::now()
        } else {
            chrono::DateTime::parse_from_rfc3339(&value).map(|dt| dt.with_timezone(&chrono::Utc))?
        };
        builder = builder.datetime(&key, dt);
    }

    for item in tags {
        let (key, value) = parse_key_value(&item)?;
        let tag_list: Vec<String> = value.split(',').map(|s| s.trim().to_string()).collect();
        builder = builder.tags(&key, tag_list);
    }

    for item in enum_value {
        let (key, rest) = parse_key_value(&item)?;
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Enum format: key=value:option1,option2,...");
        }
        let value = parts[0].to_string();
        let options: Vec<String> = parts[1].split(',').map(|s| s.trim().to_string()).collect();
        builder = builder.enum_value(&key, value, options)?;
    }

    for item in qualitative {
        let (key, rest) = parse_key_value(&item)?;
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Qualitative format: key=value:scale1,scale2,...");
        }
        let value = parts[0].to_string();
        let scale: Vec<String> = parts[1].split(',').map(|s| s.trim().to_string()).collect();
        builder = builder.qualitative(&key, value, scale)?;
    }

    for item in quantitative {
        let (key, rest) = parse_key_value(&item)?;
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Quantitative format: key=value:unit");
        }
        let value: f64 = parts[0].parse()?;
        let unit = parts[1].to_string();
        builder = builder.quantitative(&key, value, unit);
    }

    let metadata = builder.build();
    db.add_metadata(docid, &metadata)?;
    println!("Added metadata to document: {}", docid);
    Ok(())
}

fn run_get(db: &Database, docid: &str, format: OutputFormat) -> Result<()> {
    let metadata = db.get_metadata(docid)?;

    match metadata {
        Some(meta) => match format {
            OutputFormat::Json => {
                let json = meta.to_json()?;
                println!("{}", json);
            }
            _ => {
                println!("User Metadata for: {}", docid);
                println!();
                for (key, value) in &meta.fields {
                    println!("{}: {}", key, format_metadata_value(value));
                }
            }
        },
        None => {
            println!("No user metadata found for document: {}", docid);
        }
    }

    Ok(())
}

fn run_remove(db: &Database, docid: &str, fields: Vec<String>) -> Result<()> {
    db.remove_metadata_fields(docid, &fields)?;
    println!("Removed {} field(s) from document: {}", fields.len(), docid);
    Ok(())
}

fn run_clear(db: &Database, docid: &str) -> Result<()> {
    db.clear_metadata(docid)?;
    println!("Cleared all user metadata from document: {}", docid);
    Ok(())
}

fn run_query(db: &Database, filter_str: &str, limit: usize, format: OutputFormat) -> Result<()> {
    let filter = parse_filter(filter_str)?;
    let docids = db.find_by_metadata(&filter, limit)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&docids)?;
            println!("{}", json);
        }
        _ => {
            if docids.is_empty() {
                println!("No documents found matching filter: {}", filter_str);
            } else {
                println!("Found {} document(s):", docids.len());
                for docid in docids {
                    println!("  {}", docid);
                }
            }
        }
    }

    Ok(())
}

fn parse_key_value(input: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = input.splitn(2, '=').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid format: {}. Expected key=value", input);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn format_metadata_value(value: &MetadataValue) -> String {
    match value {
        MetadataValue::Text(s) => s.clone(),
        MetadataValue::Integer(n) => n.to_string(),
        MetadataValue::Float(f) => f.to_string(),
        MetadataValue::Boolean(b) => b.to_string(),
        MetadataValue::DateTime(dt) => dt.clone(),
        MetadataValue::Tags(tags) => tags.join(", "),
        MetadataValue::Enum { value, options } => {
            format!("{} (options: {})", value, options.join(", "))
        }
        MetadataValue::Qualitative { value, scale } => {
            format!("{} (scale: {})", value, scale.join(", "))
        }
        MetadataValue::Quantitative { value, unit } => {
            format!("{} {}", value, unit)
        }
        MetadataValue::Json(json) => {
            serde_json::to_string(json).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

fn parse_filter(filter_str: &str) -> Result<MetadataFilter> {
    let parts: Vec<&str> = filter_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid filter format. Expected field:operator=value");
    }

    let field = parts[0].to_string();
    let operation = parts[1];

    let op_parts: Vec<&str> = operation.splitn(2, '=').collect();
    if op_parts.len() != 2 {
        anyhow::bail!("Invalid filter format. Expected field:operator=value");
    }

    let operator = op_parts[0];
    let value = op_parts[1];

    match operator {
        "eq" => Ok(MetadataFilter::TextEq(field, value.to_string())),
        "contains" => Ok(MetadataFilter::TextContains(field, value.to_string())),
        "gt" => {
            if let Ok(num) = value.parse::<i64>() {
                Ok(MetadataFilter::IntegerGt(field, num))
            } else if let Ok(num) = value.parse::<f64>() {
                Ok(MetadataFilter::FloatGt(field, num))
            } else {
                anyhow::bail!("Invalid numeric value for gt: {}", value)
            }
        }
        "lt" => {
            if let Ok(num) = value.parse::<i64>() {
                Ok(MetadataFilter::IntegerLt(field, num))
            } else if let Ok(num) = value.parse::<f64>() {
                Ok(MetadataFilter::FloatLt(field, num))
            } else {
                anyhow::bail!("Invalid numeric value for lt: {}", value)
            }
        }
        "after" => Ok(MetadataFilter::DateTimeAfter(field, value.to_string())),
        "before" => Ok(MetadataFilter::DateTimeBefore(field, value.to_string())),
        "has" => Ok(MetadataFilter::TagsContain(field, value.to_string())),
        "exists" => Ok(MetadataFilter::Exists(field)),
        _ => anyhow::bail!(
            "Unknown operator: {}. Supported: eq, contains, gt, lt, after, before, has, exists",
            operator
        ),
    }
}
