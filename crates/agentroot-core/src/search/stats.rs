//! Search performance statistics with atomic counters

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Debug)]
pub enum QueryType {
    Bm25,
    Vector,
    Hybrid,
}

pub struct SearchStats {
    bm25_queries: AtomicU64,
    vector_queries: AtomicU64,
    hybrid_queries: AtomicU64,
    total_latency_us: AtomicU64,
    query_count: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    ann_searches: AtomicU64,
    bruteforce_searches: AtomicU64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchStatsSnapshot {
    pub bm25_queries: u64,
    pub vector_queries: u64,
    pub hybrid_queries: u64,
    pub avg_latency_us: u64,
    pub cache_hit_rate: f64,
    pub ann_searches: u64,
    pub bruteforce_searches: u64,
}

impl SearchStats {
    pub fn new() -> Self {
        Self {
            bm25_queries: AtomicU64::new(0),
            vector_queries: AtomicU64::new(0),
            hybrid_queries: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            query_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            ann_searches: AtomicU64::new(0),
            bruteforce_searches: AtomicU64::new(0),
        }
    }

    pub fn record_query(&self, query_type: QueryType, latency: Duration) {
        match query_type {
            QueryType::Bm25 => self.bm25_queries.fetch_add(1, Ordering::Relaxed),
            QueryType::Vector => self.vector_queries.fetch_add(1, Ordering::Relaxed),
            QueryType::Hybrid => self.hybrid_queries.fetch_add(1, Ordering::Relaxed),
        };
        self.total_latency_us
            .fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
        self.query_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ann_search(&self) {
        self.ann_searches.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_bruteforce_search(&self) {
        self.bruteforce_searches.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> SearchStatsSnapshot {
        let total_queries = self.query_count.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);

        let avg_latency_us = if total_queries > 0 {
            total_latency / total_queries
        } else {
            0
        };

        let total_cache = hits + misses;
        let cache_hit_rate = if total_cache > 0 {
            hits as f64 / total_cache as f64
        } else {
            0.0
        };

        SearchStatsSnapshot {
            bm25_queries: self.bm25_queries.load(Ordering::Relaxed),
            vector_queries: self.vector_queries.load(Ordering::Relaxed),
            hybrid_queries: self.hybrid_queries.load(Ordering::Relaxed),
            avg_latency_us,
            cache_hit_rate,
            ann_searches: self.ann_searches.load(Ordering::Relaxed),
            bruteforce_searches: self.bruteforce_searches.load(Ordering::Relaxed),
        }
    }
}

impl Default for SearchStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_snapshot() {
        let stats = SearchStats::new();

        stats.record_query(QueryType::Bm25, Duration::from_micros(100));
        stats.record_query(QueryType::Vector, Duration::from_micros(200));
        stats.record_query(QueryType::Hybrid, Duration::from_micros(300));
        stats.record_cache_hit();
        stats.record_cache_hit();
        stats.record_cache_miss();
        stats.record_ann_search();
        stats.record_bruteforce_search();
        stats.record_bruteforce_search();

        let snap = stats.snapshot();
        assert_eq!(snap.bm25_queries, 1);
        assert_eq!(snap.vector_queries, 1);
        assert_eq!(snap.hybrid_queries, 1);
        assert_eq!(snap.avg_latency_us, 200); // (100+200+300)/3
        assert!((snap.cache_hit_rate - 0.6667).abs() < 0.01);
        assert_eq!(snap.ann_searches, 1);
        assert_eq!(snap.bruteforce_searches, 2);
    }

    #[test]
    fn test_snapshot_empty() {
        let stats = SearchStats::new();
        let snap = stats.snapshot();
        assert_eq!(snap.bm25_queries, 0);
        assert_eq!(snap.avg_latency_us, 0);
        assert!((snap.cache_hit_rate - 0.0).abs() < 0.001);
    }
}
