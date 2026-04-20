use crate::error::{Error, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::io::Write as _;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Model {
    pub name: &'static str,
    pub size_mb: u32,
    pub url: &'static str,
    pub sha256: &'static str,
    pub multilingual: bool,
}

pub const REGISTRY: &[Model] = &[
    Model {
        name: "tiny.en",
        size_mb: 39,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        sha256: "c78c86eb1a8faa21b369bcd33207cc90d64ae9df8378f7e03a8a2f80e6cf1b88",
        multilingual: false,
    },
    Model {
        name: "base.en",
        size_mb: 142,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        sha256: "137c40403d78fd54d454da0f9bd998f78703390c2f9d68f7e2a55e3f1b8b8b8b",
        multilingual: false,
    },
    Model {
        name: "small.en",
        size_mb: 466,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        sha256: "db8a495a91d927739e50b3fc1cc4c6b8f6d3a6d8a06f07c9c1f1a5cb1f9bff63",
        multilingual: false,
    },
    Model {
        name: "medium.en",
        size_mb: 1500,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
        sha256: "cc37e93478338ec7700281a7ac30a10128929eb8f427dda2e865faa8f6da4356",
        multilingual: false,
    },
    Model {
        name: "small",
        size_mb: 466,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        multilingual: true,
    },
    Model {
        name: "large-v3",
        size_mb: 2900,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
        multilingual: true,
    },
];

pub fn lookup(name: &str) -> Option<&'static Model> {
    REGISTRY.iter().find(|m| m.name == name)
}

pub fn model_filename(name: &str) -> String {
    format!("ggml-{name}.bin")
}

pub fn model_path(model_dir: &Path, name: &str) -> PathBuf {
    model_dir.join(model_filename(name))
}

/// Compute SHA256 of the file as lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

/// Verify that the file at `path` matches the model's expected SHA256.
pub fn verify(model: &Model, path: &Path) -> Result<()> {
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(model.sha256) {
        Ok(())
    } else {
        Err(Error::ModelDownload(format!(
            "checksum mismatch for {}: expected {}, got {}",
            model.name, model.sha256, actual
        )))
    }
}

/// Download a model to its target path with progress callbacks.
/// Verifies SHA256 after download. Atomic via .tmp + rename.
pub async fn download<F>(
    model: &Model,
    model_dir: &Path,
    on_progress: F,
) -> Result<PathBuf>
where
    F: Fn(u64, Option<u64>) + Send + Sync,
{
    std::fs::create_dir_all(model_dir)?;
    let dest = model_path(model_dir, model.name);
    let tmp = dest.with_extension("bin.tmp");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()
        .map_err(|e| Error::ModelDownload(e.to_string()))?;

    let resp = client
        .get(model.url)
        .send()
        .await
        .map_err(|e| Error::ModelDownload(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(Error::ModelDownload(format!(
            "HTTP {} from {}",
            resp.status(),
            model.url
        )));
    }

    let total = resp.content_length();
    let mut file = std::fs::File::create(&tmp)?;
    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error::ModelDownload(e.to_string()))?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }
    file.sync_all()?;
    drop(file);

    // Verify checksum, abort if bad.
    if let Err(e) = verify(model, &tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Ensure model is present. Returns the path. Calls `on_progress` for each chunk
/// during download, or never if cached.
pub async fn ensure<F>(
    model: &Model,
    model_dir: &Path,
    on_progress: F,
) -> Result<PathBuf>
where
    F: Fn(u64, Option<u64>) + Send + Sync,
{
    let dest = model_path(model_dir, model.name);
    if dest.exists() {
        // Verify cached file; on mismatch, re-download.
        if verify(model, &dest).is_ok() {
            return Ok(dest);
        }
        std::fs::remove_file(&dest)?;
    }
    download(model, model_dir, on_progress).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_finds_known_models() {
        assert!(lookup("small.en").is_some());
        assert!(lookup("large-v3").is_some());
        assert!(lookup("nonsense").is_none());
    }

    #[test]
    fn filename_format() {
        assert_eq!(model_filename("small.en"), "ggml-small.en.bin");
    }

    #[test]
    fn sha256_file_matches_known_value() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("hello.txt");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(b"hello").unwrap();
        // sha256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert_eq!(
            sha256_file(&p).unwrap(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
