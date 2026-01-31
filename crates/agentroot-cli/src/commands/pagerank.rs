//! PageRank computation command

use agentroot_core::Database;
use anyhow::Result;

/// Compute PageRank scores for all documents
pub async fn run(db: &Database) -> Result<()> {
    println!("Computing PageRank scores...");
    println!();
    
    println!("Step 1: Building document link graph...");
    let link_count = db.build_link_graph()?;
    println!("  Found {} links between documents", link_count);
    println!();
    
    println!("Step 2: Running PageRank algorithm...");
    db.compute_and_store_pagerank()?;
    println!("  PageRank scores computed and stored");
    println!();
    
    let (doc_count, top_docs) = db.get_pagerank_stats()?;
    
    println!("Results:");
    println!("  Documents scored: {}", doc_count);
    println!();
    
    println!("Top 10 most important documents:");
    for (i, (path, score)) in top_docs.iter().enumerate() {
        println!("  {:2}. {:.2}  {}", i + 1, score, path);
    }
    
    Ok(())
}
