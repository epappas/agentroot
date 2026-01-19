//! MCP resource handlers

use agentroot_core::Database;
use crate::protocol::ResourceContent;
use anyhow::Result;

/// Read a resource by URI
#[allow(dead_code)]
pub async fn read_resource(db: &Database, uri: &str) -> Result<ResourceContent> {
    // Parse agentroot:// URI
    if !uri.starts_with("agentroot://") {
        anyhow::bail!("Invalid URI: {}", uri);
    }

    let rest = &uri["agentroot://".len()..];
    let parts: Vec<&str> = rest.splitn(2, '/').collect();

    if parts.len() < 2 {
        anyhow::bail!("Invalid URI format: {}", uri);
    }

    let collection = parts[0];
    let path = parts[1];

    let doc = db.find_active_document(collection, path)?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", uri))?;

    let content = db.get_content(&doc.hash)?
        .ok_or_else(|| anyhow::anyhow!("Content not found for: {}", uri))?;

    Ok(ResourceContent {
        uri: uri.to_string(),
        name: format!("{}/{}", collection, path),
        title: Some(doc.title),
        mime_type: "text/markdown".to_string(),
        text: content,
    })
}
