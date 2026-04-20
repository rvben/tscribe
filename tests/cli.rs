use assert_cmd::Command;
use predicates::prelude::*;

fn cmd() -> Command {
    Command::cargo_bin("tscribe").unwrap()
}

#[test]
fn version_flag_prints_semver() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("tscribe "));
}

#[test]
fn help_flag_lists_subcommands() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("cache"))
        .stdout(predicate::str::contains("models"))
        .stdout(predicate::str::contains("doctor"));
}

#[test]
fn doctor_exits_zero() {
    cmd()
        .env("TSCRIBE_CACHE_DIR", tempfile::tempdir().unwrap().path())
        .arg("doctor")
        .assert()
        .success();
}

#[test]
fn cache_path_prints_dir() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .env("TSCRIBE_CACHE_DIR", dir.path())
        .arg("cache")
        .arg("path")
        .assert()
        .success()
        .stdout(predicate::str::contains(dir.path().to_str().unwrap()));
}

#[test]
fn cache_list_empty_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .env("TSCRIBE_CACHE_DIR", dir.path())
        .arg("cache")
        .arg("list")
        .assert()
        .success();
}

#[test]
fn unknown_format_errors() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .env("TSCRIBE_CACHE_DIR", dir.path())
        .args(["https://example.com", "-f", "xml"])
        .assert()
        .failure();
}

#[test]
fn missing_url_prints_help() {
    cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}
