//! Indexing pipeline
//!
//! File scanning, parsing, and chunking for document indexing.

pub mod ast_chunker;
mod chunker;
mod embedder;
mod parser;
mod scanner;

pub use ast_chunker::{chunk_semantic, ChunkType, SemanticChunk, SemanticChunker};
pub use chunker::*;
pub use embedder::*;
pub use parser::*;
pub use scanner::*;
