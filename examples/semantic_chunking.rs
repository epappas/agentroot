// Semantic chunking example demonstrating AST-aware code chunking

use agentroot_core::SemanticChunker;
use std::path::Path;

fn main() -> agentroot_core::Result<()> {
    println!("Agentroot Semantic Chunking Example\n");

    // Create semantic chunker
    let chunker = SemanticChunker::new();

    // Sample Rust code
    let rust_code = r#"
//! User management module

use std::collections::HashMap;

/// Represents a user in the system.
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
}

impl User {
    /// Creates a new user with the given details.
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self { id, name, email }
    }

    /// Validates the user's email format.
    pub fn validate_email(&self) -> bool {
        self.email.contains('@') && self.email.contains('.')
    }
}

/// User database manager.
pub struct UserManager {
    users: HashMap<u64, User>,
}

impl UserManager {
    /// Creates a new empty user manager.
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    /// Adds a user to the database.
    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    /// Finds a user by ID.
    pub fn find_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
}
"#;

    // Chunk the code
    println!("Chunking Rust code...\n");
    let path = Path::new("user.rs");
    let chunks = chunker.chunk(rust_code, path)?;

    println!("Extracted {} semantic chunks:\n", chunks.len());

    for (i, chunk) in chunks.iter().enumerate() {
        println!("Chunk {}:", i + 1);
        println!("  Type: {:?}", chunk.chunk_type);
        println!("  Language: {:?}", chunk.metadata.language);
        println!(
            "  Lines: {}-{}",
            chunk.metadata.start_line, chunk.metadata.end_line
        );

        if let Some(breadcrumb) = &chunk.metadata.breadcrumb {
            println!("  Breadcrumb: {}", breadcrumb);
        }

        // Show first line of text
        let first_line = chunk.text.lines().next().unwrap_or("");
        println!("  Text: {}...", first_line.trim());

        // Show leading trivia (comments/docstrings)
        if !chunk.metadata.leading_trivia.is_empty() {
            let trivia_preview = chunk.metadata.leading_trivia.lines().next().unwrap_or("");
            println!("  Docstring: {}...", trivia_preview.trim());
        }

        // Show chunk hash
        println!("  Hash: {}", chunk.chunk_hash);

        println!();
    }

    // Demonstrate chunk types
    println!("Chunk type breakdown:");
    use agentroot_core::ChunkType as CT;
    let mut counts = [0; 10]; // One for each ChunkType variant
    for chunk in &chunks {
        let idx = match chunk.chunk_type {
            CT::Function => 0,
            CT::Method => 1,
            CT::Class => 2,
            CT::Struct => 3,
            CT::Enum => 4,
            CT::Trait => 5,
            CT::Interface => 6,
            CT::Module => 7,
            CT::Import => 8,
            CT::Text => 9,
        };
        counts[idx] += 1;
    }

    let types = [
        ("Function", counts[0]),
        ("Method", counts[1]),
        ("Class", counts[2]),
        ("Struct", counts[3]),
        ("Enum", counts[4]),
        ("Trait", counts[5]),
        ("Interface", counts[6]),
        ("Module", counts[7]),
        ("Import", counts[8]),
        ("Text", counts[9]),
    ];

    for (type_name, count) in types.iter() {
        if *count > 0 {
            println!("  {}: {}", type_name, count);
        }
    }

    // Sample Python code
    println!("\n---\n");
    println!("Chunking Python code...\n");

    let python_code = r#"
"""User management module."""

class User:
    """Represents a user in the system."""
    
    def __init__(self, id, name, email):
        """Initialize a new user."""
        self.id = id
        self.name = name
        self.email = email
    
    def validate_email(self):
        """Validate email format."""
        return '@' in self.email and '.' in self.email

class UserManager:
    """Manages user database."""
    
    def __init__(self):
        """Initialize empty user database."""
        self.users = {}
    
    def add_user(self, user):
        """Add a user to the database."""
        self.users[user.id] = user
    
    def find_user(self, user_id):
        """Find a user by ID."""
        return self.users.get(user_id)
"#;

    let python_path = Path::new("user.py");
    let python_chunks = chunker.chunk(python_code, python_path)?;

    println!("Extracted {} Python chunks:\n", python_chunks.len());

    for (i, chunk) in python_chunks.iter().enumerate() {
        println!(
            "Chunk {}: {:?} at lines {}-{}",
            i + 1,
            chunk.chunk_type,
            chunk.metadata.start_line,
            chunk.metadata.end_line
        );

        if let Some(breadcrumb) = &chunk.metadata.breadcrumb {
            println!("  Breadcrumb: {}", breadcrumb);
        }
    }

    // Demonstrate hash stability
    println!("\n---\n");
    println!("Demonstrating chunk hash stability...\n");

    let code1 = "fn foo() { bar() }";
    let code2 = "fn foo() { bar() }"; // Identical
    let code3 = "fn foo() { baz() }"; // Different

    let path = Path::new("test.rs");
    let chunks1 = chunker.chunk(code1, path)?;
    let chunks2 = chunker.chunk(code2, path)?;
    let chunks3 = chunker.chunk(code3, path)?;

    let hash1 = &chunks1[0].chunk_hash;
    let hash2 = &chunks2[0].chunk_hash;
    let hash3 = &chunks3[0].chunk_hash;

    println!("Code 1 hash: {}", hash1);
    println!("Code 2 hash: {}", hash2);
    println!("Code 3 hash: {}", hash3);
    println!();
    println!("Hash 1 == Hash 2 (identical): {}", hash1 == hash2);
    println!("Hash 1 == Hash 3 (different): {}", hash1 == hash3);

    println!("\nThis demonstrates content-addressable caching:");
    println!("- Identical code produces identical hashes");
    println!("- Different code produces different hashes");
    println!("- Cache can reuse embeddings for unchanged chunks");

    println!("\nExample completed successfully!");

    Ok(())
}
