//! Test caching and metrics in VLLMClient

use agentroot_core::{LLMClient, VLLMClient};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create client
    let client = VLLMClient::from_env()?;

    println!("=== Test 1: Embed same text twice ===");
    println!("First embed (cache miss expected):");
    let start = std::time::Instant::now();
    let _emb1 = client.embed("test text for caching").await?;
    println!("Time: {:?}", start.elapsed());

    println!("\nSecond embed (cache hit expected):");
    let start = std::time::Instant::now();
    let _emb2 = client.embed("test text for caching").await?;
    println!("Time: {:?}", start.elapsed());

    println!("\nThird embed (different text, cache miss expected):");
    let start = std::time::Instant::now();
    let _emb3 = client.embed("different text for testing").await?;
    println!("Time: {:?}", start.elapsed());

    println!("\n=== Metrics ===");
    let metrics = client.metrics();
    println!("Total Requests: {}", metrics.total_requests);
    println!("Cache Hits: {}", metrics.cache_hits);
    println!("Cache Misses: {}", metrics.cache_misses);
    println!("Cache Hit Rate: {:.1}%", metrics.cache_hit_rate);
    println!("Avg Latency: {:.1}ms", metrics.avg_latency_ms);

    Ok(())
}
