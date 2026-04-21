//! End-to-end pipeline test against a stable Creative Commons clip.
//!
//! Gated behind `--ignored` because it:
//! - requires network
//! - requires yt-dlp + ffmpeg installed
//! - downloads ~40 MB of audio
//! - downloads tiny.en model (~39 MB) on first run
//! - takes 30-60 seconds
//!
//! Run with: `cargo nextest run --test e2e --run-ignored all`
use assert_cmd::Command;
use predicates::prelude::*;

const SHORT_CC_CLIP: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

#[test]
#[ignore]
fn e2e_transcribe_to_markdown_stdout() {
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("tscribe")
        .unwrap()
        .env("TSCRIBE_CACHE_DIR", dir.path())
        .args([SHORT_CC_CLIP, "-m", "tiny.en", "-q"])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("source: "))
        .stdout(predicate::str::contains("model: \"tiny.en\""));
}

#[test]
#[ignore]
fn e2e_cache_hit_is_instant() {
    let dir = tempfile::tempdir().unwrap();
    let bin = || {
        let mut c = Command::cargo_bin("tscribe").unwrap();
        c.env("TSCRIBE_CACHE_DIR", dir.path()).args([
            SHORT_CC_CLIP,
            "-m",
            "tiny.en",
            "-q",
            "-f",
            "json",
        ]);
        c
    };

    bin().assert().success();
    let started = std::time::Instant::now();
    bin().assert().success();
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_secs() < 5,
        "second run should hit cache, took {elapsed:?}"
    );
}
