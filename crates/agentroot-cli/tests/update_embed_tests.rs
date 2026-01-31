//! Integration tests for update and embed commands

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn agentroot_cmd() -> Command {
    Command::cargo_bin("agentroot").unwrap()
}

fn setup_collection() -> (TempDir, TempDir) {
    let test_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();

    let test_files = vec![
        ("file1.md", "# Document 1\n\nFirst document content."),
        ("file2.md", "# Document 2\n\nSecond document content."),
        ("file3.md", "# Document 3\n\nThird document content."),
    ];

    for (path, content) in &test_files {
        let full_path = test_dir.path().join(path);
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
        .arg("testcollection");
    add_cmd.assert().success();

    (test_dir, db_dir)
}

#[test]
fn test_update_indexes_files() {
    let (_test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Indexed").or(predicate::str::contains("files")));
}

#[test]
fn test_update_specific_collection() {
    let (_test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("testcollection").or(predicate::str::contains("files")));
}

#[test]
fn test_update_no_collections() {
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No collections"));
}

#[test]
fn test_update_incremental() {
    let (test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut first_update = agentroot_cmd();
    first_update
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");
    first_update.assert().success();

    fs::write(
        test_dir.path().join("file4.md"),
        "# Document 4\n\nNew file.",
    )
    .unwrap();

    let mut second_update = agentroot_cmd();
    second_update
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");
    second_update.assert().success();

    let mut search = agentroot_cmd();
    search
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("search")
        .arg("Document 4");
    search.assert().success();
}

#[test]
fn test_update_with_verbose() {
    let (_test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update")
        .arg("--verbose");

    cmd.assert().success();
}

#[test]
fn test_update_shows_progress() {
    let (_test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Indexed").or(predicate::str::contains("files")));
}

#[test]
fn test_update_empty_collection() {
    let empty_dir = TempDir::new().unwrap();
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut add_cmd = agentroot_cmd();
    add_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("collection")
        .arg("add")
        .arg(empty_dir.path())
        .arg("--name")
        .arg("empty");
    add_cmd.assert().success();

    let mut update_cmd = agentroot_cmd();
    update_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");

    update_cmd.assert().success();
}

#[test]
fn test_status_after_update() {
    let (_test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut update_cmd = agentroot_cmd();
    update_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");
    update_cmd.assert().success();

    let mut status_cmd = agentroot_cmd();
    status_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("status");

    status_cmd
        .assert()
        .success()
        .stdout(predicate::str::is_match("Collections:\\s+1").unwrap())
        .stdout(predicate::str::is_match("Documents:\\s+3").unwrap());
}

#[test]
fn test_status_shows_document_count() {
    let (_test_dir, db_dir) = setup_collection();
    let db_path = db_dir.path().join("test.sqlite");

    let mut update_cmd = agentroot_cmd();
    update_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("update");
    update_cmd.assert().success();

    let mut status_cmd = agentroot_cmd();
    status_cmd
        .env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("status");

    status_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("3").or(predicate::str::contains("documents")));
}

#[test]
fn test_status_empty_database() {
    let db_dir = TempDir::new().unwrap();
    let db_path = db_dir.path().join("test.sqlite");

    let mut cmd = agentroot_cmd();
    cmd.env("AGENTROOT_DB", db_path.to_str().unwrap())
        .arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_match("Collections:\\s+0").unwrap());
}
