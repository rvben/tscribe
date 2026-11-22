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

#[test]
fn invalid_thread_counts_are_rejected_before_transcription() {
    let dir = tempfile::tempdir().unwrap();
    for count in ["0", "2147483648"] {
        cmd()
            .env("TSCRIBE_CACHE_DIR", dir.path())
            .args([
                "https://example.com/audio.wav",
                "--model",
                "unknown-model",
                "--threads",
                count,
            ])
            .assert()
            .code(2)
            .stderr(
                predicate::str::contains("invalid value")
                    .and(predicate::str::contains("--threads")),
            );
    }
}

#[test]
fn no_cache_and_refresh_conflict() {
    let dir = tempfile::tempdir().unwrap();
    cmd()
        .env("TSCRIBE_CACHE_DIR", dir.path())
        .args([
            "https://example.com/audio.wav",
            "--model",
            "unknown-model",
            "--no-cache",
            "--refresh",
        ])
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("cannot be used with")
                .and(predicate::str::contains("--refresh")),
        );
}

#[test]
fn reads_v025_transcript_and_index_without_media_tools() {
    const LEGACY_TRANSCRIPT: &str = include_str!("fixtures/transcript-v0.2.5.json");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("cache");
    let url = "https://example.com/hour.wav";
    let key = tscribe::cache::Cache::key(url, "small.en", "en");
    let transcripts = root.join("transcripts");
    let shard = transcripts.join(&key[..2]);
    std::fs::create_dir_all(&shard).unwrap();
    std::fs::write(root.join("version"), "1").unwrap();
    std::fs::write(shard.join(format!("{key}.json")), LEGACY_TRANSCRIPT).unwrap();
    let index = serde_json::json!({
        "entries": {
            &key: {
                "url": url,
                "title": "hour",
                "model": "small.en",
                "language": "en",
                "transcribed_at": "2026-11-22T15:06:37.407433Z"
            }
        }
    });
    std::fs::write(transcripts.join("index.json"), index.to_string()).unwrap();

    let mut command = cmd();
    command
        .env("PATH", dir.path())
        .env("TSCRIBE_CACHE_DIR", &root)
        .args([
            url,
            "--model",
            "small.en",
            "--lang",
            "en",
            "--no-download",
            "-q",
            "-f",
            "json",
        ]);
    let result = command.assert().success();
    let actual: serde_json::Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    let expected: serde_json::Value = serde_json::from_str(LEGACY_TRANSCRIPT).unwrap();
    assert_eq!(actual, expected);

    let result = cmd()
        .env("TSCRIBE_CACHE_DIR", &root)
        .args(["cache", "list", "--json"])
        .assert()
        .success();
    let list: serde_json::Value = serde_json::from_slice(&result.get_output().stdout).unwrap();
    assert_eq!(list["total"], 1);
    assert_eq!(list["items"][0]["url"], url);
    assert_eq!(list["items"][0]["date"], "2026-11-22");
}
