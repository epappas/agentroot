//! LLaMA-based metadata generator using llama-cpp-2

use super::metadata_generator::{DocumentMetadata, MetadataContext, MetadataGenerator};
use crate::error::{AgentRootError, Result};
use crate::index::ast_chunker::ChunkType;
use async_trait::async_trait;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, LlamaModel},
};
use std::path::Path;
use std::sync::Mutex;

/// Default metadata generation model
pub const DEFAULT_METADATA_MODEL: &str = "llama-3.1-8b-instruct.Q4_K_M.gguf";

/// Maximum tokens to send to LLM
const MAX_CONTENT_TOKENS: usize = 2048;

/// LLaMA-based metadata generator
pub struct LlamaMetadataGenerator {
    #[allow(dead_code)]
    backend: LlamaBackend,
    model: LlamaModel,
    context: Mutex<LlamaMetadataContext>,
    model_name: String,
}

struct LlamaMetadataContext {
    ctx: llama_cpp_2::context::LlamaContext<'static>,
}

unsafe impl Send for LlamaMetadataContext {}
unsafe impl Sync for LlamaMetadataContext {}

impl LlamaMetadataGenerator {
    /// Create a new LlamaMetadataGenerator from a GGUF model file
    pub fn new(model_path: impl AsRef<Path>) -> Result<Self> {
        let model_path = model_path.as_ref();
        let model_name = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut backend = LlamaBackend::init()
            .map_err(|e| AgentRootError::Llm(format!("Failed to init backend: {}", e)))?;
        backend.void_logs();

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| AgentRootError::Llm(format!("Failed to load model: {}", e)))?;

        let ctx_size = std::num::NonZeroU32::new(4096).unwrap();
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(ctx_size))
            .with_n_batch(ctx_size.get())
            .with_n_ubatch(ctx_size.get());

        let ctx = model
            .new_context(&backend, ctx_params)
            .map_err(|e| AgentRootError::Llm(format!("Failed to create context: {}", e)))?;

        let ctx: llama_cpp_2::context::LlamaContext<'static> = unsafe { std::mem::transmute(ctx) };

        Ok(Self {
            backend,
            model,
            context: Mutex::new(LlamaMetadataContext { ctx }),
            model_name,
        })
    }

    /// Create from default model location
    pub fn from_default() -> Result<Self> {
        let model_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("agentroot")
            .join("models");

        let model_path = model_dir.join(DEFAULT_METADATA_MODEL);

        if !model_path.exists() {
            return Err(AgentRootError::ModelNotFound(format!(
                "Model not found at {}. Download an instruction-tuned model (e.g., llama-3.1-8b-instruct) to this location.",
                model_path.display()
            )));
        }

        Self::new(model_path)
    }

    /// Extract key sections from large documents using smart strategies
    fn extract_key_sections(&self, content: &str, context: &MetadataContext) -> String {
        let est_tokens = content.len() / 4;

        if est_tokens <= MAX_CONTENT_TOKENS {
            return content.to_string();
        }

        match context.file_extension.as_deref() {
            Some("md") | Some("markdown") => self.extract_markdown_sections(content),
            Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") => {
                self.extract_code_sections(content, context)
            }
            _ => self.extract_generic_sections(content),
        }
    }

    /// Extract key sections from markdown (headers + first paragraph of each section)
    fn extract_markdown_sections(&self, content: &str) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() && result.len() < MAX_CONTENT_TOKENS * 4 {
            let line = lines[i];

            if line.starts_with('#') {
                result.push_str(line);
                result.push('\n');

                i += 1;
                while i < lines.len() && !lines[i].is_empty() {
                    result.push_str(lines[i]);
                    result.push('\n');
                    i += 1;
                    if result.len() >= MAX_CONTENT_TOKENS * 4 {
                        break;
                    }
                }
            } else if i == 0 && !line.is_empty() {
                result.push_str(line);
                result.push('\n');
            }
            i += 1;
        }

        if result.is_empty() {
            content.chars().take(MAX_CONTENT_TOKENS * 4).collect()
        } else {
            result
        }
    }

    /// Extract key sections from code (function/class signatures + docstrings)
    fn extract_code_sections(&self, content: &str, context: &MetadataContext) -> String {
        let mut result = String::new();
        let lines: Vec<&str> = content.lines().collect();

        for line in lines.iter().take(50) {
            result.push_str(line);
            result.push('\n');
        }

        if let Some(structure) = &context.existing_structure {
            result.push_str("\n\nStructure: ");
            let types: Vec<String> = structure.iter().map(|ct| ct.as_str().to_string()).collect();
            result.push_str(&types.join(", "));
        }

        if result.len() > MAX_CONTENT_TOKENS * 4 {
            result.truncate(MAX_CONTENT_TOKENS * 4);
        }

        result
    }

    /// Extract generic sections (first and last portions)
    fn extract_generic_sections(&self, content: &str) -> String {
        let half_size = (MAX_CONTENT_TOKENS * 4) / 2;
        let chars: Vec<char> = content.chars().collect();

        if chars.len() <= MAX_CONTENT_TOKENS * 4 {
            return content.to_string();
        }

        let first: String = chars.iter().take(half_size).collect();
        let last: String = chars.iter().skip(chars.len() - half_size).collect();

        format!("{}\n\n... [content truncated] ...\n\n{}", first, last)
    }

    /// Build prompt for metadata generation
    fn build_prompt(&self, content: &str, context: &MetadataContext) -> String {
        let structure_info = if let Some(structure) = &context.existing_structure {
            let types: Vec<String> = structure.iter().map(|ct| ct.as_str().to_string()).collect();
            format!("Code structures: {}", types.join(", "))
        } else {
            "No structural information available".to_string()
        };

        format!(
            r#"You are a document analysis assistant. Analyze the following document and provide comprehensive metadata.

DOCUMENT INFORMATION:
- Source: {}
- Language: {}
- Collection: {}
- File Type: {}
- {}

DOCUMENT CONTENT:
{}

Generate the following metadata as valid JSON:
1. summary: A 100-200 word summary of the main content and purpose
2. semantic_title: A clear, descriptive title (improve upon filename if needed)
3. keywords: Array of 5-10 relevant keywords/tags for search
4. category: Document category (choose one: tutorial, reference, configuration, test, documentation, code, research, api, guide, example)
5. intent: Brief description of why this document exists and what problem it solves
6. concepts: Array of key concepts, technologies, frameworks, or entities mentioned
7. difficulty: Target audience level (choose one: beginner, intermediate, advanced)
8. suggested_queries: Array of 3-5 search queries a user might type to find this document

Respond ONLY with valid JSON in this exact format (no markdown, no code blocks):
{{"summary":"...","semantic_title":"...","keywords":["..."],"category":"...","intent":"...","concepts":["..."],"difficulty":"...","suggested_queries":["..."]}}
"#,
            context.source_type,
            context.language.as_deref().unwrap_or("unknown"),
            context.collection_name,
            context.file_extension.as_deref().unwrap_or("unknown"),
            structure_info,
            content
        )
    }

    /// Parse LLM response into structured metadata
    fn parse_metadata_response(&self, response: &str) -> Result<DocumentMetadata> {
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let start = cleaned.find('{').unwrap_or(0);
        let end = cleaned.rfind('}').map(|i| i + 1).unwrap_or(cleaned.len());
        let json_str = &cleaned[start..end];

        serde_json::from_str::<DocumentMetadata>(json_str).map_err(|e| {
            AgentRootError::Llm(format!(
                "Failed to parse metadata JSON: {}. Response was: {}",
                e, json_str
            ))
        })
    }

    /// Generate text using LLM
    fn generate_sync(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let mut ctx_guard = self
            .context
            .lock()
            .map_err(|e| AgentRootError::Llm(format!("Lock error: {}", e)))?;

        let tokens = self
            .model
            .str_to_token(prompt, llama_cpp_2::model::AddBos::Always)
            .map_err(|e| AgentRootError::Llm(format!("Tokenization error: {}", e)))?;

        if tokens.is_empty() {
            return Err(AgentRootError::Llm("Empty tokenization".to_string()));
        }

        let mut batch = LlamaBatch::new(tokens.len(), 1);

        for (i, token) in tokens.iter().enumerate() {
            batch
                .add(*token, i as i32, &[0], false)
                .map_err(|e| AgentRootError::Llm(format!("Batch error: {}", e)))?;
        }

        ctx_guard
            .ctx
            .decode(&mut batch)
            .map_err(|e| AgentRootError::Llm(format!("Decode error: {}", e)))?;

        let mut generated = String::new();
        let mut token_count = 0;

        loop {
            if token_count >= max_tokens {
                break;
            }

            let candidates: Vec<_> = ctx_guard.ctx.candidates().collect();
            if candidates.is_empty() {
                break;
            }

            let token = candidates[0].id();

            if token == self.model.token_eos() {
                break;
            }

            let token_str = self
                .model
                .token_to_str(token, llama_cpp_2::model::Special::Tokenize)
                .map_err(|e| AgentRootError::Llm(format!("Token to string error: {}", e)))?;

            generated.push_str(&token_str);
            token_count += 1;

            let mut new_batch = LlamaBatch::new(1, 1);
            new_batch
                .add(
                    token,
                    tokens.len() as i32 + token_count as i32 - 1,
                    &[0],
                    false,
                )
                .map_err(|e| AgentRootError::Llm(format!("Batch add error: {}", e)))?;

            ctx_guard
                .ctx
                .decode(&mut new_batch)
                .map_err(|e| AgentRootError::Llm(format!("Decode error: {}", e)))?;
        }

        Ok(generated)
    }

    /// Generate metadata using LLM with fallback
    async fn generate_with_fallback(
        &self,
        content: &str,
        context: &MetadataContext,
    ) -> DocumentMetadata {
        let truncated_content = self.extract_key_sections(content, context);
        let prompt = self.build_prompt(&truncated_content, context);

        match self.generate_sync(&prompt, 512) {
            Ok(response) => match self.parse_metadata_response(&response) {
                Ok(metadata) => metadata,
                Err(e) => {
                    eprintln!("Failed to parse LLM response: {}. Using fallback.", e);
                    self.generate_fallback_metadata(content, context)
                }
            },
            Err(e) => {
                eprintln!("LLM generation failed: {}. Using fallback.", e);
                self.generate_fallback_metadata(content, context)
            }
        }
    }

    /// Generate fallback metadata using heuristics
    fn generate_fallback_metadata(
        &self,
        content: &str,
        context: &MetadataContext,
    ) -> DocumentMetadata {
        let title = self.improve_title_from_path(&context.collection_name);
        let summary = self.extract_first_paragraph(content);
        let keywords = self.extract_keywords_basic(content);
        let category = self.infer_category_from_context(context);
        let concepts = self.extract_capitalized_terms(content);
        let difficulty = "intermediate".to_string();
        let suggested_queries = vec![title.clone()];

        DocumentMetadata {
            summary,
            semantic_title: title.clone(),
            keywords,
            category,
            intent: format!("Document from {} collection", context.collection_name),
            concepts,
            difficulty,
            suggested_queries,
        }
    }

    /// Improve title from file path
    fn improve_title_from_path(&self, path: &str) -> String {
        path.split('/')
            .next_back()
            .unwrap_or(path)
            .replace(['-', '_'], " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Extract first paragraph as summary
    fn extract_first_paragraph(&self, content: &str) -> String {
        let mut summary = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//") {
                summary.push_str(trimmed);
                summary.push(' ');
                if summary.len() > 200 {
                    break;
                }
            }
            if summary.len() > 100 && trimmed.is_empty() {
                break;
            }
        }
        if summary.is_empty() {
            content.chars().take(200).collect()
        } else {
            summary.truncate(200);
            summary
        }
    }

    /// Extract basic keywords from content
    fn extract_keywords_basic(&self, content: &str) -> Vec<String> {
        let words: Vec<&str> = content
            .split_whitespace()
            .filter(|w| w.len() > 4 && w.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .take(50)
            .collect();

        let mut word_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for word in words {
            *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }

        let mut sorted: Vec<_> = word_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted.into_iter().take(8).map(|(word, _)| word).collect()
    }

    /// Infer category from context
    fn infer_category_from_context(&self, context: &MetadataContext) -> String {
        if let Some(structure) = &context.existing_structure {
            if structure
                .iter()
                .any(|ct| matches!(ct, ChunkType::Function | ChunkType::Method))
            {
                return "code".to_string();
            }
        }

        match context.file_extension.as_deref() {
            Some("md") | Some("markdown") => "documentation".to_string(),
            Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") => "code".to_string(),
            Some("json") | Some("yaml") | Some("toml") | Some("ini") => "configuration".to_string(),
            Some("txt") => "documentation".to_string(),
            _ => "documentation".to_string(),
        }
    }

    /// Extract capitalized terms as concepts
    fn extract_capitalized_terms(&self, content: &str) -> Vec<String> {
        let mut concepts = std::collections::HashSet::new();

        for word in content.split_whitespace().take(200) {
            let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();

            if clean.len() > 2 && clean.chars().next().is_some_and(|c| c.is_uppercase()) {
                concepts.insert(clean);
            }
        }

        concepts.into_iter().take(10).collect()
    }
}

#[async_trait]
impl MetadataGenerator for LlamaMetadataGenerator {
    async fn generate_metadata(
        &self,
        content: &str,
        context: &MetadataContext,
    ) -> Result<DocumentMetadata> {
        Ok(self.generate_with_fallback(content, context).await)
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_improve_title_from_path() {
        let title = improve_title_from_path_standalone("my-test-file");
        assert_eq!(title, "My Test File");
        let title2 = improve_title_from_path_standalone("some_document");
        assert_eq!(title2, "Some Document");
    }

    #[test]
    fn test_extract_first_paragraph_standalone() {
        let content = "This is the first paragraph.\n\nThis is the second paragraph.";
        let result = extract_first_paragraph_standalone(content);
        assert!(result.contains("first paragraph"));
    }

    #[test]
    fn test_extract_keywords_basic_standalone() {
        let content = "testing testing hello world testing hello";
        let keywords = extract_keywords_basic_standalone(content);
        assert!(keywords.contains(&"testing".to_string()));
    }

    #[test]
    fn test_parse_metadata_json() {
        let json = r#"{"summary":"Test summary","semantic_title":"Test Title","keywords":["test","rust"],"category":"code","intent":"Testing","concepts":["Unit Testing"],"difficulty":"beginner","suggested_queries":["rust testing"]}"#;
        let metadata = serde_json::from_str::<DocumentMetadata>(json).unwrap();
        assert_eq!(metadata.semantic_title, "Test Title");
        assert_eq!(metadata.category, "code");
    }

    fn improve_title_from_path_standalone(path: &str) -> String {
        path.split('/')
            .last()
            .unwrap_or(path)
            .replace('-', " ")
            .replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn extract_first_paragraph_standalone(content: &str) -> String {
        let mut summary = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//") {
                summary.push_str(trimmed);
                summary.push(' ');
                if summary.len() > 200 {
                    break;
                }
            }
            if summary.len() > 100 && trimmed.is_empty() {
                break;
            }
        }
        if summary.is_empty() {
            content.chars().take(200).collect()
        } else {
            summary.truncate(200);
            summary
        }
    }

    fn extract_keywords_basic_standalone(content: &str) -> Vec<String> {
        let words: Vec<&str> = content
            .split_whitespace()
            .filter(|w| w.len() > 4 && w.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .take(50)
            .collect();

        let mut word_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for word in words {
            *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
        }

        let mut sorted: Vec<_> = word_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted.into_iter().take(8).map(|(word, _)| word).collect()
    }
}
