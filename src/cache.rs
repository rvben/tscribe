use crate::config::Paths;
use crate::error::Result;
use crate::transcript::{SCHEMA_VERSION, TranscriptEntry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Index {
    pub entries: BTreeMap<String, IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub url: String,
    pub title: Option<String>,
    pub model: String,
    pub language: String,
    pub transcribed_at: chrono::DateTime<chrono::Utc>,
}

pub struct Cache {
    paths: Paths,
}

impl Cache {
    pub fn new(paths: Paths) -> Result<Self> {
        fs::create_dir_all(&paths.transcript_dir)?;
        ensure_version_file(&paths)?;
        Ok(Self { paths })
    }

    /// Compute SHA256 of (url || \0 || model || \0 || lang) as lowercase hex.
    pub fn key(url: &str, model: &str, lang: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hasher.update([0u8]);
        hasher.update(model.as_bytes());
        hasher.update([0u8]);
        hasher.update(lang.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn entry_path(&self, key: &str) -> PathBuf {
        let shard = &key[..2];
        self.paths.transcript_dir.join(shard).join(format!("{key}.json"))
    }

    pub fn get(&self, key: &str) -> Result<Option<TranscriptEntry>> {
        let path = self.entry_path(key);
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path)?;
        match serde_json::from_str::<TranscriptEntry>(&data) {
            Ok(entry) if entry.version == SCHEMA_VERSION => Ok(Some(entry)),
            _ => Ok(None), // corrupt or schema-mismatched entries treated as miss
        }
    }

    pub fn put(&self, key: &str, entry: &TranscriptEntry) -> Result<()> {
        let path = self.entry_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("json.tmp");
        let mut f = fs::File::create(&tmp)?;
        f.write_all(serde_json::to_string_pretty(entry)?.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, &path)?;
        self.update_index(key, entry)?;
        Ok(())
    }

    fn update_index(&self, key: &str, entry: &TranscriptEntry) -> Result<()> {
        let index_path = self.paths.transcript_dir.join("index.json");
        let mut index: Index = if index_path.exists() {
            serde_json::from_str(&fs::read_to_string(&index_path)?).unwrap_or_default()
        } else {
            Index::default()
        };
        index.entries.insert(
            key.to_string(),
            IndexEntry {
                url: entry.url.clone(),
                title: entry.metadata.title.clone(),
                model: entry.transcription.model.clone(),
                language: entry.transcription.language.clone(),
                transcribed_at: entry.transcription.transcribed_at,
            },
        );
        let tmp = index_path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(&index)?)?;
        fs::rename(&tmp, &index_path)?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<(String, IndexEntry)>> {
        let index_path = self.paths.transcript_dir.join("index.json");
        if !index_path.exists() {
            return Ok(Vec::new());
        }
        let index: Index = serde_json::from_str(&fs::read_to_string(&index_path)?)?;
        Ok(index.entries.into_iter().collect())
    }

    pub fn clear(&self) -> Result<()> {
        if self.paths.transcript_dir.exists() {
            fs::remove_dir_all(&self.paths.transcript_dir)?;
        }
        fs::create_dir_all(&self.paths.transcript_dir)?;
        Ok(())
    }
}

fn ensure_version_file(paths: &Paths) -> Result<()> {
    fs::create_dir_all(&paths.cache_dir)?;
    let current = SCHEMA_VERSION.to_string();
    if let Ok(existing) = fs::read_to_string(&paths.version_file)
        && existing.trim() != current
    {
        // Schema mismatch — wipe transcripts (models survive).
        if paths.transcript_dir.exists() {
            fs::remove_dir_all(&paths.transcript_dir)?;
            fs::create_dir_all(&paths.transcript_dir)?;
        }
    }
    fs::write(&paths.version_file, current)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Paths;
    use tempfile::tempdir;

    fn fixture_entry() -> TranscriptEntry {
        const FIXTURE: &str = include_str!("../tests/fixtures/transcript.json");
        serde_json::from_str(FIXTURE).unwrap()
    }

    #[test]
    fn key_is_deterministic() {
        let k1 = Cache::key("https://example.com", "small.en", "en");
        let k2 = Cache::key("https://example.com", "small.en", "en");
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 64);
    }

    #[test]
    fn key_differs_per_input() {
        let base = Cache::key("https://example.com", "small.en", "en");
        assert_ne!(base, Cache::key("https://example.com/other", "small.en", "en"));
        assert_ne!(base, Cache::key("https://example.com", "small", "en"));
        assert_ne!(base, Cache::key("https://example.com", "small.en", "nl"));
    }

    #[test]
    fn put_then_get_returns_entry() {
        let dir = tempdir().unwrap();
        let cache = Cache::new(Paths::with_root(dir.path().to_path_buf())).unwrap();
        let entry = fixture_entry();
        let key = Cache::key(&entry.url, &entry.transcription.model, &entry.transcription.language);
        cache.put(&key, &entry).unwrap();
        let loaded = cache.get(&key).unwrap().unwrap();
        assert_eq!(loaded, entry);
    }

    #[test]
    fn missing_key_returns_none() {
        let dir = tempdir().unwrap();
        let cache = Cache::new(Paths::with_root(dir.path().to_path_buf())).unwrap();
        assert!(cache.get("0".repeat(64).as_str()).unwrap().is_none());
    }

    #[test]
    fn corrupt_entry_treated_as_miss() {
        let dir = tempdir().unwrap();
        let cache = Cache::new(Paths::with_root(dir.path().to_path_buf())).unwrap();
        let key = "0".repeat(64);
        let path = cache.entry_path(&key);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();
        assert!(cache.get(&key).unwrap().is_none());
    }

    #[test]
    fn list_returns_index_entries() {
        let dir = tempdir().unwrap();
        let cache = Cache::new(Paths::with_root(dir.path().to_path_buf())).unwrap();
        let entry = fixture_entry();
        let key = Cache::key(&entry.url, &entry.transcription.model, &entry.transcription.language);
        cache.put(&key, &entry).unwrap();
        let list = cache.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].1.url, entry.url);
    }

    #[test]
    fn clear_wipes_transcripts() {
        let dir = tempdir().unwrap();
        let cache = Cache::new(Paths::with_root(dir.path().to_path_buf())).unwrap();
        let entry = fixture_entry();
        let key = Cache::key(&entry.url, &entry.transcription.model, &entry.transcription.language);
        cache.put(&key, &entry).unwrap();
        cache.clear().unwrap();
        assert!(cache.get(&key).unwrap().is_none());
    }

    #[test]
    fn schema_mismatch_wipes_cache() {
        let dir = tempdir().unwrap();
        let paths = Paths::with_root(dir.path().to_path_buf());
        let cache = Cache::new(paths.clone()).unwrap();
        let entry = fixture_entry();
        let key = Cache::key(&entry.url, &entry.transcription.model, &entry.transcription.language);
        cache.put(&key, &entry).unwrap();

        // Simulate older schema on disk.
        fs::write(&paths.version_file, "0").unwrap();

        let _ = Cache::new(paths.clone()).unwrap();
        let cache2 = Cache::new(paths).unwrap();
        assert!(cache2.get(&key).unwrap().is_none());
    }
}
