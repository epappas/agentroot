//! Search commands

use crate::app::{OutputFormat, SearchArgs};
use crate::output::{format_search_results, FormatOptions};
use agentroot_core::{smart_search, CandleEmbedder, Database, SearchOptions};
use anyhow::Result;

pub async fn run_bm25(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");
    let options = build_options(&args);

    let results = db.search_fts(&query, &options)?;

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!("{}", format_search_results(&results, format, &format_opts));
    Ok(())
}

pub async fn run_vector(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");
    let options = build_options(&args);

    // Check if vector index exists
    if !db.has_vector_index() {
        eprintln!("Warning: No vector embeddings found. Run 'agentroot embed' first.");
        eprintln!("Falling back to BM25 search.");
        return run_bm25(args, db, format).await;
    }

    // Load embedder
    let embedder = match load_embedder() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Warning: Could not load embedding model: {}", e);
            eprintln!("Falling back to BM25 search.");
            return run_bm25(args, db, format).await;
        }
    };

    let results = db.search_vec(&query, &embedder, &options).await?;

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!("{}", format_search_results(&results, format, &format_opts));
    Ok(())
}

pub async fn run_hybrid(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");
    let options = build_options(&args);

    // Check if vector index exists
    if !db.has_vector_index() {
        eprintln!("Warning: No vector embeddings found. Run 'agentroot embed' first.");
        eprintln!("Running BM25 search only.");
        return run_bm25(args, db, format).await;
    }

    // Load embedder
    let embedder = match load_embedder() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Warning: Could not load embedding model: {}", e);
            eprintln!("Running BM25 search only.");
            return run_bm25(args, db, format).await;
        }
    };

    // Run hybrid search (BM25 + Vector with RRF fusion)
    let bm25_results = db.search_fts(&query, &options)?;
    let vec_results = db.search_vec(&query, &embedder, &options).await?;

    // RRF fusion
    let results = agentroot_core::search::rrf_fusion(&bm25_results, &vec_results);

    // Apply limit and min_score
    let final_results: Vec<_> = results
        .into_iter()
        .filter(|r| r.score >= options.min_score)
        .take(options.limit)
        .collect();

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!(
        "{}",
        format_search_results(&final_results, format, &format_opts)
    );
    Ok(())
}

fn build_options(args: &SearchArgs) -> SearchOptions {
    SearchOptions {
        limit: if args.all { usize::MAX } else { args.limit },
        min_score: args.min_score,
        collection: args.collection.clone(),
        provider: None,
        full_content: args.full,
    }
}

pub async fn run_smart(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");
    let options = build_options(&args);

    // Smart search handles fallbacks internally
    let results = smart_search(db, &query, &options).await?;

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!("{}", format_search_results(&results, format, &format_opts));
    Ok(())
}

fn load_embedder() -> Result<CandleEmbedder> {
    Ok(CandleEmbedder::from_default()?)
}
