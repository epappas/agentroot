//! Integration tests for search commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn agentroot_cmd() -> Command {
    Command::cargo_bin("agentroot").unwrap()
}

fn setup_test_collection() -> (TempDir, TempDir) {
    let test_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();

    let test_files = vec![
        (
            "error_handling.md",
            "# Error Handling\n\nUse Result<T> for error propagation.\n\n```rust\nfn process() -> Result<()> {\n    let data = fetch()?;\n    Ok(())\n}\n```",
        ),
        (
            "async.md",
            "# Async Programming\n\nUse async/await for concurrent operations.\n\n```rust\nasync fn fetch_data() -> Result<Data> {\n    let response = client.get(url).await?;\n    Ok(response)\n}\n```",
        ),
        (
            "testing.md",
            "# Testing Guide\n\nWrite comprehensive tests for all functionality.\n\n```rust\n#[test]\nfn test_parsing() {\n    assert_eq!(parse(\"test\"), expected);\n}\n```",
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
        .arg("testdocs");
    add_cmd.assert().success();

    let mut update_cmd = agentroot_cmd();
    update_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");
    update_cmd.assert().success();

    (test_dir, db_dir)
}

#[test]
fn test_search_runs_successfully() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("search")
        .arg("error handling");

    cmd.assert().success();
}

#[test]
fn test_search_with_limit() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("search")
        .arg("rust")
        .arg("-n")
        .arg("1");

    cmd.assert().success();
}

#[test]
fn test_search_with_min_score() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("search")
        .arg("async")
        .arg("--min-score")
        .arg("0.5");

    cmd.assert().success();
}

#[test]
fn test_search_with_collection_filter() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("search")
        .arg("testing")
        .arg("--collection")
        .arg("testdocs");

    cmd.assert().success();
}

#[test]
fn test_search_output_json() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("--format")
        .arg("json")
        .arg("search")
        .arg("error");

    cmd.assert().success();
}

#[test]
fn test_search_output_csv() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("--format")
        .arg("csv")
        .arg("search")
        .arg("error");

    cmd.assert().success();
}

#[test]
fn test_search_output_markdown() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("--format")
        .arg("md")
        .arg("search")
        .arg("error");

    cmd.assert().success();
}

#[test]
fn test_query_runs_successfully() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("query")
        .arg("async programming");

    cmd.assert().success();
}

#[test]
fn test_status_shows_collections() {
    let (_test_dir, db_dir) = setup_test_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Collections: 1"));
}
