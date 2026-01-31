//! Document graph and PageRank computation

mod link_extractor;
mod pagerank;

pub use link_extractor::extract_links;
pub use pagerank::compute_pagerank;
