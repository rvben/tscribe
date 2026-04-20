use directories::ProjectDirs;
use std::path::PathBuf;

pub const DEFAULT_MODEL_EN: &str = "small.en";
pub const DEFAULT_MODEL_MULTI: &str = "small";
pub const DEFAULT_LANG: &str = "en";

#[derive(Debug, Clone)]
pub struct Paths {
    pub cache_dir: PathBuf,
    pub model_dir: PathBuf,
    pub transcript_dir: PathBuf,
    pub version_file: PathBuf,
}

impl Paths {
    /// Resolve paths from env vars or platform defaults.
    pub fn discover() -> Self {
        let cache_dir = std::env::var_os("TSCRIBE_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(default_cache_dir);

        let model_dir = std::env::var_os("TSCRIBE_MODEL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| cache_dir.join("models"));

        let transcript_dir = cache_dir.join("transcripts");
        let version_file = cache_dir.join("version");

        Self {
            cache_dir,
            model_dir,
            transcript_dir,
            version_file,
        }
    }

    /// Override base cache dir (used in tests).
    pub fn with_root(root: PathBuf) -> Self {
        Self {
            model_dir: root.join("models"),
            transcript_dir: root.join("transcripts"),
            version_file: root.join("version"),
            cache_dir: root,
        }
    }
}

fn default_cache_dir() -> PathBuf {
    ProjectDirs::from("nl", "am8", "tscribe")
        .map(|p| p.cache_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./.tscribe-cache"))
}

/// Choose the appropriate default model for a language code.
pub fn default_model_for_lang(lang: &str) -> &'static str {
    if lang == "en" {
        DEFAULT_MODEL_EN
    } else {
        DEFAULT_MODEL_MULTI
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_picks_english_for_en() {
        assert_eq!(default_model_for_lang("en"), "small.en");
    }

    #[test]
    fn default_model_picks_multi_for_other_langs() {
        assert_eq!(default_model_for_lang("nl"), "small");
        assert_eq!(default_model_for_lang("fr"), "small");
    }

    #[test]
    fn paths_with_root_compose_subdirs() {
        let root = std::path::PathBuf::from("/tmp/x");
        let p = Paths::with_root(root.clone());
        assert_eq!(p.cache_dir, root);
        assert_eq!(p.model_dir, root.join("models"));
        assert_eq!(p.transcript_dir, root.join("transcripts"));
    }
}
