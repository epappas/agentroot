//! Indexing pipeline
//!
//! File scanning, parsing, and chunking for document indexing.

mod scanner;
mod parser;
mod chunker;
mod embedder;
pub mod ast_chunker;

pub use scanner::*;
pub use parser::*;
pub use chunker::*;
pub use embedder::*;
pub use ast_chunker::{SemanticChunk, ChunkType, chunk_semantic, SemanticChunker};
