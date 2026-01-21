//! HTTP-based metadata generator using external LLM service

use super::{ChatMessage, DocumentMetadata, LLMClient, MetadataContext, MetadataGenerator};
use crate::config::LLMServiceConfig;
use crate::error::{AgentRootError, Result};
use async_trait::async_trait;
use std::sync::Arc;

/// Metadata generator using external HTTP LLM service
pub struct HttpMetadataGenerator {
    client: Arc<dyn LLMClient>,
}

impl HttpMetadataGenerator {
    /// Create from LLM client
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        Self { client }
    }

    /// Create from configuration
    pub fn from_config(config: LLMServiceConfig) -> Result<Self> {
        let client = super::VLLMClient::new(config)?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let client = super::VLLMClient::from_env()?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Build prompt for metadata extraction
    fn build_metadata_prompt(&self, content: &str, context: &MetadataContext) -> String {
        // Truncate content intelligently based on source type
        let truncated = self.truncate_content(content, context);

        format!(
            r#"Extract metadata from this document and output ONLY valid JSON.

Document Info:
- Source: {} 
- Language: {}
- Collection: {}

Content:
{}

Output JSON with these exact fields:
{{
  "summary": "100-200 word summary",
  "semantic_title": "improved title", 
  "keywords": ["keyword1", "keyword2", "keyword3", "keyword4", "keyword5"],
  "category": "document type (code/documentation/tutorial/reference/guide)",
  "intent": "purpose description",
  "concepts": ["concept1", "concept2", "concept3"],
  "difficulty": "beginner/intermediate/advanced",
  "suggested_queries": ["query1", "query2", "query3"]
}}

JSON:"#,
            context.source_type,
            context.language.as_deref().unwrap_or("unknown"),
            context.collection_name,
            truncated
        )
    }

    /// Truncate content intelligently based on type
    fn truncate_content(&self, content: &str, context: &MetadataContext) -> String {
        const MAX_CHARS: usize = 8000;

        if content.len() <= MAX_CHARS {
            return content.to_string();
        }

        // For markdown, extract headers and first paragraph of each section
        if context.file_extension.as_deref() == Some("md") {
            return self.truncate_markdown(content, MAX_CHARS);
        }

        // For code, extract structure + docstrings
        if matches!(
            context.language.as_deref(),
            Some("rust") | Some("python") | Some("javascript") | Some("typescript")
        ) {
            return self.truncate_code(content, MAX_CHARS);
        }

        // Default: first + last portions
        let half = MAX_CHARS / 2;
        format!(
            "{}\n\n[... truncated ...]\n\n{}",
            &content[..half.min(content.len())],
            &content[content.len().saturating_sub(half)..]
        )
    }

    fn truncate_markdown(&self, content: &str, max_chars: usize) -> String {
        let mut result = String::new();
        let mut current_len = 0;

        for line in content.lines() {
            if current_len >= max_chars {
                break;
            }

            // Always include headers or non-empty lines that fit
            let should_include =
                line.starts_with('#') || (!line.is_empty() && current_len + line.len() < max_chars);

            if should_include {
                result.push_str(line);
                result.push('\n');
                current_len += line.len() + 1;
            }
        }

        result
    }

    fn truncate_code(&self, content: &str, max_chars: usize) -> String {
        let mut result = String::new();
        let mut current_len = 0;
        let mut in_comment = false;

        for line in content.lines() {
            if current_len >= max_chars {
                break;
            }

            let trimmed = line.trim();

            // Include function/class signatures
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("export ")
                || trimmed.contains("struct ")
                || trimmed.contains("impl ")
            {
                result.push_str(line);
                result.push('\n');
                current_len += line.len() + 1;
            }
            // Include comments and docstrings
            else if trimmed.starts_with("//")
                || trimmed.starts_with('#')
                || trimmed.starts_with("///")
                || trimmed.starts_with("\"\"\"")
                || trimmed.starts_with("/*")
            {
                result.push_str(line);
                result.push('\n');
                current_len += line.len() + 1;
                in_comment = trimmed.starts_with("/*");
            } else if in_comment {
                result.push_str(line);
                result.push('\n');
                current_len += line.len() + 1;
                if trimmed.ends_with("*/") {
                    in_comment = false;
                }
            }
        }

        result
    }

    /// Parse JSON response from LLM
    fn parse_metadata_response(&self, response: &str) -> Result<DocumentMetadata> {
        // Extract JSON from response (handle markdown code blocks and extra text)
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            return Err(AgentRootError::Llm(
                "No JSON found in LLM response".to_string(),
            ));
        };

        serde_json::from_str(json_str)
            .map_err(|e| AgentRootError::Llm(format!("Failed to parse metadata JSON: {}", e)))
    }
}

#[async_trait]
impl MetadataGenerator for HttpMetadataGenerator {
    async fn generate_metadata(
        &self,
        content: &str,
        context: &MetadataContext,
    ) -> Result<DocumentMetadata> {
        let prompt = self.build_metadata_prompt(content, context);

        let messages = vec![
            ChatMessage::system(
                "You are a metadata extraction expert. Analyze documents and output structured JSON metadata. \
                 Be concise, accurate, and output ONLY valid JSON with no additional text."
            ),
            ChatMessage::user(prompt),
        ];

        let response = self.client.chat_completion(messages).await?;
        self.parse_metadata_response(&response)
    }

    fn model_name(&self) -> &str {
        self.client.model_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_markdown() {
        let generator =
            HttpMetadataGenerator::new(Arc::new(super::super::VLLMClient::from_env().unwrap()));

        let content = r#"# Title
Some intro

## Section 1
Content here

## Section 2  
More content"#;

        let context = MetadataContext {
            source_type: "file".to_string(),
            language: None,
            file_extension: Some("md".to_string()),
            collection_name: "test".to_string(),
            provider_config: None,
            created_at: "".to_string(),
            modified_at: "".to_string(),
            existing_structure: None,
        };

        let truncated = generator.truncate_content(content, &context);
        assert!(truncated.contains("# Title"));
        assert!(truncated.contains("## Section 1"));
    }
}
