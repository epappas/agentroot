//! Integration tests for document commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn agentroot_cmd() -> Command {
    Command::cargo_bin("agentroot").unwrap()
}

fn setup_indexed_collection() -> (TempDir, TempDir) {
    let test_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();

    let test_files = vec![
        (
            "README.md",
            "# Test Project\n\nWelcome to the test project.",
        ),
        (
            "src/lib.rs",
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
        ),
        (
            "src/main.rs",
            "fn main() {\n    println!(\"Hello, world!\");\n}",
        ),
        (
            "docs/api.md",
            "# API Documentation\n\n## Functions\n\n### add\n\nAdds two numbers.",
        ),
    ];

    for (path, content) in &test_files {
        let full_path = test_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    let db_path = db_dir.path().join("test.sqlite");

    let mut add_cmd = agentroot_cmd();
    add_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(test_dir.path())
        .arg("--name")
        .arg("testproject");
    add_cmd.assert().success();

    let mut update_cmd = agentroot_cmd();
    update_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");
    update_cmd.assert().success();

    (test_dir, db_dir)
}

#[test]
fn test_get_document_by_path() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("get")
        .arg("testproject/README.md");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Project"))
        .stdout(predicate::str::contains("Welcome to the test project"));
}

#[test]
fn test_get_nonexistent_document() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("get")
        .arg("nonexistent/file.md");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("No document")));
}

#[test]
fn test_get_with_line_range() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("get")
        .arg("testproject/src/lib.rs")
        .arg("--from-line")
        .arg("1")
        .arg("--max-lines")
        .arg("1");

    cmd.assert().success();
}

#[test]
fn test_get_with_line_numbers() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("get")
        .arg("testproject/README.md")
        .arg("--line-numbers");

    cmd.assert().success().stdout(predicate::str::contains("1"));
}

#[test]
fn test_ls_collection() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("ls")
        .arg("testproject");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("docs/api.md"));
}

#[test]
fn test_ls_all_collections() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap()).arg("ls");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("testproject"));
}

#[test]
fn test_ls_nonexistent_collection() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("ls")
        .arg("nonexistent");

    cmd.assert().failure().stderr(
        predicate::str::contains("not found").or(predicate::str::contains("No collection")),
    );
}

#[test]
fn test_multi_get_by_pattern() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("multi-get")
        .arg("testproject/src/*.rs");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("lib.rs").or(predicate::str::contains("main.rs")));
}

#[test]
fn test_multi_get_with_max_lines() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("multi-get")
        .arg("testproject/**/*.md")
        .arg("--max-lines")
        .arg("5");

    cmd.assert().success();
}

#[test]
fn test_multi_get_with_line_numbers() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("multi-get")
        .arg("testproject/*.md")
        .arg("--line-numbers");

    cmd.assert().success();
}

#[test]
fn test_multi_get_comma_separated() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("multi-get")
        .arg("testproject/README.md,testproject/docs/api.md");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("README").or(predicate::str::contains("API")));
}

#[test]
fn test_multi_get_empty_pattern() {
    let (_test_dir, db_dir) = setup_indexed_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("multi-get")
        .arg("testproject/nonexistent/*.xyz");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No documents").or(predicate::str::is_empty()));
}
