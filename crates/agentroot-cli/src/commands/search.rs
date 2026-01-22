//! Search commands

use crate::app::{OutputFormat, SearchArgs};
use crate::output::{format_search_results, FormatOptions};
use agentroot_core::{
    smart_search, unified_search, Database, Embedder, HttpEmbedder, HttpQueryExpander,
    HttpReranker, QueryExpander, Reranker, SearchOptions,
};
use anyhow::Result;

/// Unified intelligent search - automatically chooses best strategy
pub async fn run_bm25(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    let query = args.query.join(" ");
    let options = build_options(&args);

    // Check if orchestrated mode is enabled (ReAct-style workflow planning)
    let use_orchestrated = std::env::var("AGENTROOT_ORCHESTRATED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let results = if use_orchestrated {
        // Use workflow orchestration - LLM plans custom multi-step workflows
        agentroot_core::orchestrated_search(db, &query, &options).await?
    } else {
        // Use unified search - LLM picks single strategy
        unified_search(db, &query, &options).await?
    };

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!("{}", format_search_results(&results, format, &format_opts));
    Ok(())
}

pub async fn run_vector(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    eprintln!(
        "Note: 'vsearch' is deprecated. Use 'agentroot search' for automatic strategy selection."
    );
    eprintln!();

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

    let results = db.search_vec(&query, embedder.as_ref(), &options).await?;

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!("{}", format_search_results(&results, format, &format_opts));
    Ok(())
}

pub async fn run_hybrid(args: SearchArgs, db: &Database, format: OutputFormat) -> Result<()> {
    eprintln!(
        "Note: 'query' is deprecated. Use 'agentroot search' for automatic strategy selection."
    );
    eprintln!();

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

    // Load query expander (optional)
    let expander = load_query_expander();

    // Load reranker (optional)
    let reranker = load_reranker();

    // Run full hybrid search with query expansion and reranking
    let results = agentroot_core::search::hybrid_search(
        db,
        &query,
        &options,
        embedder.as_ref(),
        expander.as_ref().map(|e| e.as_ref()),
        reranker.as_ref().map(|r| r.as_ref()),
    )
    .await?;

    let format_opts = FormatOptions {
        full: args.full,
        query: Some(query),
        line_numbers: args.line_numbers,
    };

    print!("{}", format_search_results(&results, format, &format_opts));
    Ok(())
}

fn build_options(args: &SearchArgs) -> SearchOptions {
    SearchOptions {
        limit: if args.all { usize::MAX } else { args.limit },
        min_score: args.min_score,
        collection: args.collection.clone(),
        provider: None,
        full_content: args.full,
        metadata_filters: Vec::new(),
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

fn load_embedder() -> Result<Box<dyn Embedder>> {
    // Get HTTP embedder from environment variables
    match HttpEmbedder::from_env() {
        Ok(http_embedder) => Ok(Box::new(http_embedder)),
        Err(_) => Err(anyhow::anyhow!(
            "No embedding service configured. Set AGENTROOT_EMBEDDING_URL, AGENTROOT_EMBEDDING_MODEL, and AGENTROOT_EMBEDDING_DIMS environment variables. See VLLM_SETUP.md for details."
        )),
    }
}

fn load_query_expander() -> Option<Box<dyn QueryExpander>> {
    match HttpQueryExpander::from_env() {
        Ok(expander) => {
            eprintln!("Query expansion enabled with {}", expander.model_name());
            Some(Box::new(expander))
        }
        Err(_) => {
            eprintln!("Query expansion disabled (no LLM service configured)");
            None
        }
    }
}

fn load_reranker() -> Option<Box<dyn Reranker>> {
    match HttpReranker::from_env() {
        Ok(reranker) => {
            eprintln!("Reranking enabled with {}", reranker.model_name());
            Some(Box::new(reranker))
        }
        Err(_) => {
            eprintln!("Reranking disabled (no LLM service configured)");
            None
        }
    }
}
