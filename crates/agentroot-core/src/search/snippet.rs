//! Snippet extraction for search results

/// Extracted snippet with metadata
#[derive(Debug, Clone)]
pub struct Snippet {
    pub snippet: String,
    pub start_pos: usize,
    pub end_pos: usize,
}

/// Extract a relevant snippet from content
pub fn extract_snippet(
    content: &str,
    query: &str,
    max_length: Option<usize>,
    chunk_pos: Option<usize>,
) -> Snippet {
    let max_len = max_length.unwrap_or(500);

    // If content is short enough, return it all
    if content.len() <= max_len {
        return Snippet {
            snippet: content.to_string(),
            start_pos: 0,
            end_pos: content.len(),
        };
    }

    // Start from chunk position if available
    let center = chunk_pos.unwrap_or_else(|| find_query_position(content, query));

    // Calculate window
    let half_len = max_len / 2;
    let start = center.saturating_sub(half_len);
    let end = (start + max_len).min(content.len());
    let start = if end == content.len() {
        end.saturating_sub(max_len)
    } else {
        start
    };

    // Adjust to word boundaries
    let (start, end) = adjust_to_word_boundaries(content, start, end);

    let mut snippet = content[start..end].to_string();

    // Add ellipsis
    if start > 0 {
        snippet = format!("...{}", snippet.trim_start());
    }
    if end < content.len() {
        snippet = format!("{}...", snippet.trim_end());
    }

    Snippet {
        snippet,
        start_pos: start,
        end_pos: end,
    }
}

/// Find the position of query terms in content
fn find_query_position(content: &str, query: &str) -> usize {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    // Try to find exact match first
    if let Some(pos) = content_lower.find(&query_lower) {
        return pos;
    }

    // Try individual terms
    let terms: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|t| t.len() >= 3)
        .collect();

    for term in terms {
        if let Some(pos) = content_lower.find(term) {
            return pos;
        }
    }

    // Default to start
    0
}

/// Adjust positions to word boundaries
fn adjust_to_word_boundaries(content: &str, start: usize, end: usize) -> (usize, usize) {
    let bytes = content.as_bytes();

    // Find start of word
    let mut new_start = start;
    while new_start > 0
        && bytes
            .get(new_start - 1)
            .map(|&b| !b.is_ascii_whitespace())
            .unwrap_or(false)
    {
        new_start -= 1;
    }

    // Find end of word
    let mut new_end = end;
    while new_end < bytes.len()
        && bytes
            .get(new_end)
            .map(|&b| !b.is_ascii_whitespace())
            .unwrap_or(false)
    {
        new_end += 1;
    }

    (new_start, new_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_content() {
        let snippet = extract_snippet("Hello world", "hello", None, None);
        assert_eq!(snippet.snippet, "Hello world");
    }

    #[test]
    fn test_long_content() {
        let content = "a ".repeat(500);
        let snippet = extract_snippet(&content, "test", Some(100), None);
        assert!(snippet.snippet.len() <= 110); // Allow for ellipsis
    }
}
