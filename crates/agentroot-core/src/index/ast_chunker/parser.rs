//! Tree-sitter parser wrapper

use tree_sitter::{Parser, Tree, Language as TsLanguage};
use super::language::Language;
use crate::error::{Error, Result};

/// Parse source code into a tree-sitter AST
pub fn parse(source: &str, language: Language) -> Result<Tree> {
    let mut parser = Parser::new();
    let ts_language = get_tree_sitter_language(language);
    parser.set_language(&ts_language).map_err(|e| Error::Parse(e.to_string()))?;
    parser.parse(source, None).ok_or_else(|| Error::Parse("Failed to parse source".to_string()))
}

/// Get the tree-sitter language for a Language enum variant.
/// This is infallible since all Language variants have corresponding tree-sitter languages.
fn get_tree_sitter_language(language: Language) -> TsLanguage {
    match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::TypeScriptTsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust() {
        let source = "fn main() { println!(\"Hello\"); }";
        let tree = parse(source, Language::Rust).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_parse_python() {
        let source = "def main():\n    print('Hello')";
        let tree = parse(source, Language::Python).unwrap();
        assert_eq!(tree.root_node().kind(), "module");
    }

    #[test]
    fn test_parse_javascript() {
        let source = "function main() { console.log('Hello'); }";
        let tree = parse(source, Language::JavaScript).unwrap();
        assert_eq!(tree.root_node().kind(), "program");
    }

    #[test]
    fn test_parse_typescript() {
        let source = "function main(): void { console.log('Hello'); }";
        let tree = parse(source, Language::TypeScript).unwrap();
        assert_eq!(tree.root_node().kind(), "program");
    }

    #[test]
    fn test_parse_go() {
        let source = "package main\n\nfunc main() { fmt.Println(\"Hello\") }";
        let tree = parse(source, Language::Go).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }
}
