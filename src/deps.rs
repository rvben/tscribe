use crate::error::{Error, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: &'static str,
    pub binary: &'static str,
    pub install_hint: &'static str,
}

pub const YT_DLP: Dep = Dep {
    name: "yt-dlp",
    binary: "yt-dlp",
    install_hint:
        "macOS:        brew install yt-dlp\n  Debian/Ubuntu: apt install yt-dlp\n  Arch:         pacman -S yt-dlp\n  Any:          pipx install yt-dlp",
};

pub const FFMPEG: Dep = Dep {
    name: "ffmpeg",
    binary: "ffmpeg",
    install_hint:
        "macOS:        brew install ffmpeg\n  Debian/Ubuntu: apt install ffmpeg\n  Arch:         pacman -S ffmpeg",
};

/// Returns the resolved path to the dep binary, or a structured error.
pub fn require(dep: &Dep) -> Result<PathBuf> {
    which::which(dep.binary).map_err(|_| Error::MissingDep {
        name: dep.name.to_string(),
        hint: dep.install_hint.to_string(),
    })
}

/// Returns Some(path) if dep is present, None if missing. For `doctor`.
pub fn locate(dep: &Dep) -> Option<PathBuf> {
    which::which(dep.binary).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_missing_returns_structured_error() {
        let fake = Dep {
            name: "definitely-not-installed-xyz",
            binary: "definitely-not-installed-xyz",
            install_hint: "n/a",
        };
        let err = require(&fake).unwrap_err();
        assert_eq!(err.exit_code(), 5);
        assert!(format!("{err}").contains("definitely-not-installed-xyz"));
    }

    #[test]
    fn locate_missing_returns_none() {
        let fake = Dep {
            name: "definitely-not-installed-xyz",
            binary: "definitely-not-installed-xyz",
            install_hint: "n/a",
        };
        assert!(locate(&fake).is_none());
    }
}
