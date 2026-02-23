//! Oversized chunk handling via striding

use super::super::chunker::{CHUNK_OVERLAP_CHARS, CHUNK_SIZE_CHARS};
#[cfg(test)]
use super::types::ChunkType;
use super::types::{compute_chunk_hash, ChunkMetadata, SemanticChunk};

const STRIDE_SIZE: usize = CHUNK_SIZE_CHARS;
const STRIDE_OVERLAP: usize = CHUNK_OVERLAP_CHARS;
const BREAK_SEARCH_PERCENT: usize = 30;

/// Split an oversized chunk into smaller strides
pub fn split_oversized_chunk(chunk: SemanticChunk, max_chars: usize) -> Vec<SemanticChunk> {
    if chunk.text.len() <= max_chars || max_chars == 0 {
        return vec![chunk];
    }

    let mut result = Vec::new();
    let text = &chunk.text;
    let mut start = 0;
    let mut stride_idx = 0;
    let base_line = chunk.metadata.start_line;

    // Track line count incrementally to avoid O(n^2) scanning
    let mut lines_to_prev_end = 0;
    let mut prev_end = 0;

    while start < text.len() {
        let raw_end = (start + STRIDE_SIZE).min(text.len());
        let end = find_safe_boundary(text, raw_end, start);

        // Guard: ensure end > start to prevent infinite loop
        let end = if end <= start {
            find_safe_boundary_forward(text, start.saturating_add(1))
        } else {
            end
        };
        if end <= start {
            break;
        }

        let stride_text = text[start..end].to_string();
        let breadcrumb = chunk
            .metadata
            .breadcrumb
            .as_ref()
            .map(|b| format!("{}[{}]", b, stride_idx));

        let leading = if stride_idx == 0 {
            chunk.metadata.leading_trivia.clone()
        } else {
            String::new()
        };

        let is_last = end >= text.len();
        let trailing = if is_last {
            chunk.metadata.trailing_trivia.clone()
        } else {
            String::new()
        };

        let chunk_hash = compute_chunk_hash(&stride_text, &leading, &trailing);

        // Incremental line counting: only scan new portion from prev_end to end
        lines_to_prev_end += text[prev_end..end].matches('\n').count();
        let end_line = base_line + lines_to_prev_end;
        // Lines in this stride for computing start_line
        let lines_in_stride = text[start..end].matches('\n').count();
        let start_line = end_line - lines_in_stride;

        result.push(SemanticChunk {
            text: stride_text,
            chunk_type: chunk.chunk_type,
            chunk_hash,
            position: chunk.position + start,
            token_count: None,
            metadata: ChunkMetadata {
                leading_trivia: leading,
                trailing_trivia: trailing,
                breadcrumb,
                language: chunk.metadata.language,
                start_line,
                end_line,
            },
        });

        if end >= text.len() {
            break;
        }

        prev_end = end;
        let prev_start = start;
        start = end.saturating_sub(STRIDE_OVERLAP);
        start = find_safe_boundary_forward(text, start);

        // Guard: ensure forward progress to prevent infinite loop
        if start <= prev_start {
            start = find_safe_boundary_forward(text, prev_start.saturating_add(1));
            if start <= prev_start {
                break;
            }
        }

        stride_idx += 1;
    }

    result
}

/// Split all oversized chunks in a list
pub fn split_oversized_chunks(chunks: Vec<SemanticChunk>, max_chars: usize) -> Vec<SemanticChunk> {
    chunks
        .into_iter()
        .flat_map(|c| split_oversized_chunk(c, max_chars))
        .collect()
}

/// Find a char boundary at or before index, preferring natural break points
fn find_safe_boundary(s: &str, index: usize, min_index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }

    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }

    let min_index = min_index.min(i);
    let search_window = STRIDE_SIZE * BREAK_SEARCH_PERCENT / 100;
    let mut search_start = i.saturating_sub(search_window).max(min_index);
    if search_start < i && !s.is_char_boundary(search_start) {
        search_start = find_safe_boundary_forward(s, search_start);
    }
    if search_start >= i {
        return i;
    }

    if let Some(pos) = s[search_start..i].rfind("\n\n") {
        return search_start + pos + 2;
    }
    if let Some(pos) = s[search_start..i].rfind('\n') {
        return search_start + pos + 1;
    }
    if let Some(pos) = s[search_start..i].rfind(' ') {
        return search_start + pos + 1;
    }

    i
}

/// Find a char boundary at or after index
fn find_safe_boundary_forward(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Check if a chunk is oversized
pub fn is_oversized(chunk: &SemanticChunk, max_chars: usize) -> bool {
    chunk.text.len() > max_chars
}

/// Estimate token count from character count (rough 4:1 ratio)
pub fn estimate_tokens(char_count: usize) -> usize {
    char_count / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(text: &str) -> SemanticChunk {
        SemanticChunk::new(text.to_string(), ChunkType::Function, 0)
    }

    #[test]
    fn test_small_chunk_unchanged() {
        let chunk = make_chunk("fn small() {}");
        let result = split_oversized_chunk(chunk.clone(), 1000);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, chunk.text);
    }

    #[test]
    fn test_oversized_chunk_split() {
        let large_text = "x".repeat(5000);
        let chunk = make_chunk(&large_text);
        let result = split_oversized_chunk(chunk, 1000);
        assert!(result.len() > 1);
    }

    #[test]
    fn test_stride_breadcrumb() {
        let large_text = "x".repeat(5000);
        let mut chunk = make_chunk(&large_text);
        chunk.metadata.breadcrumb = Some("my_function".to_string());
        let result = split_oversized_chunk(chunk, 1000);

        assert!(result[0]
            .metadata
            .breadcrumb
            .as_ref()
            .unwrap()
            .contains("[0]"));
        if result.len() > 1 {
            assert!(result[1]
                .metadata
                .breadcrumb
                .as_ref()
                .unwrap()
                .contains("[1]"));
        }
    }

    #[test]
    fn test_is_oversized() {
        let small = make_chunk("small");
        let large = make_chunk(&"x".repeat(5000));

        assert!(!is_oversized(&small, 1000));
        assert!(is_oversized(&large, 1000));
    }

    #[test]
    fn test_zero_max_chars_returns_original() {
        let chunk = make_chunk("some text");
        let result = split_oversized_chunk(chunk.clone(), 0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, chunk.text);
    }

    #[test]
    fn test_empty_chunk() {
        let chunk = make_chunk("");
        let result = split_oversized_chunk(chunk, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].text, "");
    }

    #[test]
    fn test_exact_boundary_chunk() {
        let chunk = make_chunk(&"x".repeat(1000));
        let result = split_oversized_chunk(chunk, 1000);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_unicode_suffix_boundary_recovery() {
        // Large ascii prefix (with breaks) + long unicode run (no breaks)
        // previously could trigger non-char-boundary slicing in stride fallback.
        let mut text = "line\n".repeat(2500);
        text.push_str(&"🧪".repeat(5000));

        let chunk = make_chunk(&text);
        let result = split_oversized_chunk(chunk, 1000);

        assert!(result.len() > 1);
        assert!(result.iter().all(|c| !c.text.is_empty()));
    }

    #[test]
    fn test_find_safe_boundary_adjusts_non_boundary_search_window_start() {
        // Regression: search window start could land in the middle of a multi-byte
        // codepoint and panic on s[search_start..i] slicing.
        let text = "━🧪".repeat(4000);
        let min_index = 1;
        let mut hit = None;
        let window = STRIDE_SIZE * BREAK_SEARCH_PERCENT / 100;

        for index in 1000..text.len() {
            let mut i = index;
            while i > 0 && !text.is_char_boundary(i) {
                i -= 1;
            }
            let candidate = i.saturating_sub(window).max(min_index);
            if candidate < i && !text.is_char_boundary(candidate) {
                hit = Some((index, i, candidate));
                break;
            }
        }

        let (index, i, _candidate) = hit.expect("test must find a non-boundary window start");
        let boundary = find_safe_boundary(&text, index, min_index);
        assert!(text.is_char_boundary(boundary));
        assert!(boundary <= i);
        assert!(boundary >= min_index);
    }
}
