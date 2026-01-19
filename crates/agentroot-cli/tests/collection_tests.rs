//! Integration tests for collection commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn agentroot_cmd() -> Command {
    Command::cargo_bin("agentroot").unwrap()
}

fn create_test_files(dir: &TempDir) -> Vec<String> {
    let test_files = vec![
        ("README.md", "# Test Project\n\nThis is a test."),
        ("src/lib.rs", "pub fn hello() { println!(\"Hello\"); }"),
        ("src/main.rs", "fn main() { lib::hello(); }"),
        ("docs/guide.md", "# User Guide\n\nHow to use this."),
    ];

    for (path, content) in &test_files {
        let full_path = dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }

    test_files.iter().map(|(p, _)| p.to_string()).collect()
}

#[test]
fn test_collection_add_success() {
    let test_dir = TempDir::new().unwrap();
    create_test_files(&test_dir);
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(test_dir.path())
        .arg("--name")
        .arg("testproject")
        .arg("--mask")
        .arg("**/*.md");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Added collection"))
        .stdout(predicate::str::contains("testproject"));
}

#[test]
fn test_collection_add_duplicate_fails() {
    let test_dir = TempDir::new().unwrap();
    create_test_files(&test_dir);
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd1 = agentroot_cmd();
    cmd1.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(test_dir.path())
        .arg("--name")
        .arg("testproject");
    cmd1.assert().success();

    let mut cmd2 = agentroot_cmd();
    cmd2.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(test_dir.path())
        .arg("--name")
        .arg("testproject");

    cmd2.assert()
        .failure()
        .stderr(predicate::str::contains("UNIQUE").or(predicate::str::contains("Database error")));
}

#[test]
fn test_collection_list_empty() {
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No collections"));
}

#[test]
fn test_collection_list_shows_collections() {
    let test_dir = TempDir::new().unwrap();
    create_test_files(&test_dir);
    let db_dir = TempDir::new().unwrap();
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

    let mut list_cmd = agentroot_cmd();
    list_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("list");

    list_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("testproject"))
        .stdout(predicate::str::contains("**/*.md"));
}

#[test]
fn test_collection_remove_success() {
    let test_dir = TempDir::new().unwrap();
    create_test_files(&test_dir);
    let db_dir = TempDir::new().unwrap();
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

    let mut remove_cmd = agentroot_cmd();
    remove_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("remove")
        .arg("testproject");

    remove_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed collection"));
}

#[test]
fn test_collection_rename_success() {
    let test_dir = TempDir::new().unwrap();
    create_test_files(&test_dir);
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut add_cmd = agentroot_cmd();
    add_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(test_dir.path())
        .arg("--name")
        .arg("oldname");
    add_cmd.assert().success();

    let mut rename_cmd = agentroot_cmd();
    rename_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("rename")
        .arg("oldname")
        .arg("newname");

    rename_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Renamed collection"));

    let mut list_cmd = agentroot_cmd();
    list_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("list");

    list_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("newname"))
        .stdout(predicate::str::contains("oldname").not());
}

#[test]
fn test_collection_add_with_provider() {
    let test_dir = TempDir::new().unwrap();
    create_test_files(&test_dir);
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(test_dir.path())
        .arg("--name")
        .arg("testproject")
        .arg("--provider")
        .arg("file");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Added collection"));
}
