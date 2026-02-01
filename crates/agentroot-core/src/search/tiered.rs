//! Tiered context loading (L0/L1/L2)
//!
//! L0 = Abstract (~100 tokens): title + category + 1-sentence summary
//! L1 = Overview (~2K tokens): summary + keywords + snippet (default)
//! L2 = Full (unlimited): complete document/chunk content

use super::SearchResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    L0,
    L1,
    L2,
}

impl Default for DetailLevel {
    fn default() -> Self {
        DetailLevel::L1
    }
}

impl DetailLevel {
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("L0") | Some("l0") => DetailLevel::L0,
            Some("L2") | Some("l2") => DetailLevel::L2,
            _ => DetailLevel::L1,
        }
    }

    pub fn is_full_content(&self) -> bool {
        *self == DetailLevel::L2
    }
}

impl SearchResult {
    /// Project this result to the given detail level, stripping fields not needed.
    pub fn project(&mut self, detail: DetailLevel) {
        match detail {
            DetailLevel::L0 => {
                self.body = None;
                self.context = None;
                self.llm_keywords = None;
                self.llm_summary = self.llm_summary.take().map(|s| first_sentence(&s));
                self.chunk_summary = self.chunk_summary.take().map(|s| first_sentence(&s));
                self.chunk_purpose = None;
                self.chunk_concepts = vec![];
            }
            DetailLevel::L1 => {
                self.body = None;
                // context (snippet), llm_summary, llm_keywords kept as-is
            }
            DetailLevel::L2 => {
                // body must have been loaded by the search function
                self.context = None;
            }
        }
    }
}

fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(pos) = trimmed.find(". ") {
        trimmed[..=pos].to_string()
    } else if let Some(pos) = trimmed.find(".\n") {
        trimmed[..=pos].to_string()
    } else if trimmed.len() <= 200 {
        trimmed.to_string()
    } else {
        // Find a safe UTF-8 boundary at or before byte 200
        let boundary = truncate_boundary(trimmed, 200);
        let safe = &trimmed[..boundary];
        if let Some(pos) = safe.rfind(' ') {
            format!("{}...", &safe[..pos])
        } else {
            format!("{}...", safe)
        }
    }
}

/// Find the largest byte offset <= max_bytes that falls on a char boundary.
fn truncate_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    boundary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detail_level_from_str() {
        assert_eq!(DetailLevel::from_str_opt(Some("L0")), DetailLevel::L0);
        assert_eq!(DetailLevel::from_str_opt(Some("l0")), DetailLevel::L0);
        assert_eq!(DetailLevel::from_str_opt(Some("L1")), DetailLevel::L1);
        assert_eq!(DetailLevel::from_str_opt(Some("L2")), DetailLevel::L2);
        assert_eq!(DetailLevel::from_str_opt(None), DetailLevel::L1);
        assert_eq!(DetailLevel::from_str_opt(Some("invalid")), DetailLevel::L1);
    }

    #[test]
    fn test_first_sentence() {
        assert_eq!(first_sentence("Hello world. More text."), "Hello world.");
        assert_eq!(first_sentence("Short text"), "Short text");
        assert_eq!(first_sentence("Line one.\nLine two."), "Line one.");
    }

    #[test]
    fn test_first_sentence_multibyte_utf8() {
        // 200+ bytes of multibyte characters: each CJK char is 3 bytes
        let cjk: String = std::iter::repeat('\u{4e00}').take(80).collect(); // 240 bytes
        let result = first_sentence(&cjk);
        assert!(result.ends_with("..."));
        // Must not panic and must be valid UTF-8
        assert!(result.len() <= 210);
    }

    #[test]
    fn test_first_sentence_emoji() {
        // Each emoji is 4 bytes; 51 emojis = 204 bytes
        let emojis: String = std::iter::repeat('\u{1F600}').take(51).collect();
        let result = first_sentence(&emojis);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_boundary() {
        assert_eq!(truncate_boundary("hello", 10), 5);
        assert_eq!(truncate_boundary("hello", 3), 3);
        // 2-byte char at byte 0-1
        let s = "\u{00e9}abc"; // e-acute (2 bytes) + "abc"
        assert_eq!(truncate_boundary(s, 1), 0); // byte 1 is mid-char, back to 0
        assert_eq!(truncate_boundary(s, 2), 2); // byte 2 is char boundary
    }

    #[test]
    fn test_project_l0() {
        let mut result = make_test_result();
        result.project(DetailLevel::L0);
        assert!(result.body.is_none());
        assert!(result.context.is_none());
        assert!(result.llm_keywords.is_none());
        assert!(result.chunk_purpose.is_none());
        assert!(result.chunk_concepts.is_empty());
        // Summary should be first sentence only
        assert_eq!(result.llm_summary.as_deref(), Some("First sentence."));
    }

    #[test]
    fn test_project_l1() {
        let mut result = make_test_result();
        result.project(DetailLevel::L1);
        assert!(result.body.is_none());
        assert!(result.context.is_some());
        assert!(result.llm_keywords.is_some());
        assert!(result.llm_summary.is_some());
    }

    #[test]
    fn test_project_l2() {
        let mut result = make_test_result();
        result.body = Some("full content".to_string());
        result.project(DetailLevel::L2);
        assert!(result.body.is_some());
        assert!(result.context.is_none());
    }

    fn make_test_result() -> SearchResult {
        SearchResult {
            filepath: "agentroot://test/doc.md".to_string(),
            display_path: "test/doc.md".to_string(),
            title: "Test Doc".to_string(),
            hash: "abc123".to_string(),
            collection_name: "test".to_string(),
            modified_at: "2024-01-01T00:00:00Z".to_string(),
            body: None,
            body_length: 100,
            docid: "abc123".to_string(),
            context: Some("...some snippet...".to_string()),
            score: 0.85,
            source: crate::search::SearchSource::Bm25,
            chunk_pos: None,
            llm_summary: Some("First sentence. Second sentence.".to_string()),
            llm_title: Some("Test Title".to_string()),
            llm_keywords: Some(vec!["rust".to_string(), "search".to_string()]),
            llm_category: Some("tutorial".to_string()),
            llm_difficulty: Some("beginner".to_string()),
            user_metadata: None,
            is_chunk: false,
            chunk_hash: None,
            chunk_type: None,
            chunk_breadcrumb: None,
            chunk_start_line: None,
            chunk_end_line: None,
            chunk_language: None,
            chunk_summary: Some("Chunk first. Chunk second.".to_string()),
            chunk_purpose: Some("Does something".to_string()),
            chunk_concepts: vec!["concept1".to_string()],
            chunk_labels: std::collections::HashMap::new(),
        }
    }
}
