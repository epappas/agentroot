//! LLM response caching to reduce API calls

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

/// Cache entry with TTL
#[derive(Clone)]
struct CacheEntry {
    value: String,
    expires_at: SystemTime,
}

/// In-memory cache for LLM responses
pub struct LLMCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    default_ttl: Duration,
}

impl LLMCache {
    /// Create new cache with default TTL of 1 hour
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(3600),
        }
    }

    /// Create cache with custom TTL
    #[allow(dead_code)]
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: ttl,
        }
    }

    /// Get cached value if exists and not expired
    pub fn get(&self, key: &str) -> Option<String> {
        let entries = self.entries.read().ok()?;
        let entry = entries.get(key)?;

        if SystemTime::now() < entry.expires_at {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Set cached value with default TTL
    pub fn set(&self, key: String, value: String) -> Result<()> {
        self.set_with_ttl(key, value, self.default_ttl)
    }

    /// Set cached value with custom TTL
    pub fn set_with_ttl(&self, key: String, value: String, ttl: Duration) -> Result<()> {
        let expires_at = SystemTime::now() + ttl;
        let entry = CacheEntry { value, expires_at };

        if let Ok(mut entries) = self.entries.write() {
            entries.insert(key, entry);
        }

        Ok(())
    }

    /// Clear expired entries
    #[allow(dead_code)]
    pub fn cleanup(&self) {
        if let Ok(mut entries) = self.entries.write() {
            let now = SystemTime::now();
            entries.retain(|_, entry| now < entry.expires_at);
        }
    }

    /// Clear all entries
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub fn stats(&self) -> CacheStats {
        if let Ok(entries) = self.entries.read() {
            let now = SystemTime::now();
            let total = entries.len();
            let expired = entries.values().filter(|e| now >= e.expires_at).count();

            CacheStats {
                total_entries: total,
                expired_entries: expired,
                active_entries: total - expired,
            }
        } else {
            CacheStats::default()
        }
    }
}

impl Default for LLMCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
}

/// Generate cache key for embeddings
pub fn embedding_cache_key(model: &str, text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    model.hash(&mut hasher);
    text.hash(&mut hasher);
    format!("embed:{}:{:x}", model, hasher.finish())
}

/// Generate cache key for chat completions
pub fn chat_cache_key(model: &str, messages: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    model.hash(&mut hasher);
    messages.hash(&mut hasher);
    format!("chat:{}:{:x}", model, hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = LLMCache::new();

        cache.set("key1".to_string(), "value1".to_string()).unwrap();
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let cache = LLMCache::with_ttl(Duration::from_millis(100));

        cache.set("key1".to_string(), "value1".to_string()).unwrap();
        assert_eq!(cache.get("key1"), Some("value1".to_string()));

        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = LLMCache::with_ttl(Duration::from_millis(100));

        cache.set("key1".to_string(), "value1".to_string()).unwrap();
        cache.set("key2".to_string(), "value2".to_string()).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 2);

        std::thread::sleep(Duration::from_millis(150));
        cache.cleanup();

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn test_cache_key_generation() {
        let key1 = embedding_cache_key("model1", "text1");
        let key2 = embedding_cache_key("model1", "text1");
        let key3 = embedding_cache_key("model1", "text2");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
