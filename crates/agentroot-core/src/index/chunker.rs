//! Document chunking for embedding

/// Chunking configuration
pub const CHUNK_SIZE_TOKENS: usize = 800;
pub const CHUNK_OVERLAP_TOKENS: usize = 120;
pub const CHUNK_SIZE_CHARS: usize = 3200;
pub const CHUNK_OVERLAP_CHARS: usize = 480;

/// Document chunk
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub position: usize,
    pub token_count: Option<usize>,
}

/// Find a valid char boundary at or before the given byte index
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find a valid char boundary at or after the given byte index
fn ceil_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// Character-based chunking (fallback)
pub fn chunk_by_chars(content: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    if content.len() <= chunk_size {
        return vec![Chunk {
            text: content.to_string(),
            position: 0,
            token_count: None,
        }];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content.len() {
        let raw_end = (start + chunk_size).min(content.len());
        let end = floor_char_boundary(content, raw_end);
        let mut chunk_end = end;

        // Find natural break point in last 30%
        if end < content.len() {
            let search_start_raw = start + (chunk_size * 70 / 100);
            let search_start = ceil_char_boundary(content, search_start_raw);

            if search_start < end {
                let search_region = &content[search_start..end];

                if let Some(pos) = search_region.rfind("\n\n") {
                    chunk_end = search_start + pos + 2;
                } else if let Some(pos) = search_region.rfind(". ") {
                    chunk_end = search_start + pos + 2;
                } else if let Some(pos) = search_region.rfind('\n') {
                    chunk_end = search_start + pos + 1;
                } else if let Some(pos) = search_region.rfind(' ') {
                    chunk_end = search_start + pos + 1;
                }
            }
        }

        // Ensure chunk_end is at a char boundary
        chunk_end = floor_char_boundary(content, chunk_end);

        chunks.push(Chunk {
            text: content[start..chunk_end].to_string(),
            position: start,
            token_count: None,
        });

        if chunk_end >= content.len() {
            break;
        }

        let new_start_raw = chunk_end.saturating_sub(overlap);
        start = ceil_char_boundary(content, new_start_raw);
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_small_content() {
        let content = "Small content.";
        let chunks = chunk_by_chars(content, 100, 20);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, content);
    }

    #[test]
    fn test_chunk_preserves_paragraphs() {
        let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = chunk_by_chars(content, 30, 5);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_chunk_handles_unicode() {
        let content = "Hello 世界! This is a test with emoji 🎉 and special chars ─ here.";
        let chunks = chunk_by_chars(content, 20, 5);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
        }
    }

    #[test]
    fn test_floor_char_boundary() {
        let s = "Hello 世界";
        assert_eq!(floor_char_boundary(s, 6), 6);  // Start of 世
        assert_eq!(floor_char_boundary(s, 7), 6);  // Inside 世
        assert_eq!(floor_char_boundary(s, 8), 6);  // Inside 世
        assert_eq!(floor_char_boundary(s, 9), 9);  // Start of 界
    }
}
