use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("missing system dependency: {name} (install hint: {hint})")]
    MissingDep { name: String, hint: String },

    #[error("unsupported or invalid URL: {0}")]
    BadUrl(String),

    #[error("{0}")]
    BadArg(String),

    #[error("yt-dlp failed: {0}")]
    Download(String),

    #[error("{0}")]
    Unsupported(String),

    #[error("ffmpeg failed: {0}")]
    Audio(String),

    #[error("transcription failed: {0}")]
    Transcribe(String),

    #[error("model download failed: {0}")]
    ModelDownload(String),

    #[error("{0}")]
    ModelMissing(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Map this error to the documented exit code.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::BadUrl(_) | Error::BadArg(_) | Error::Unsupported(_) => 2,
            Error::Download(_) => 3,
            Error::Transcribe(_) => 4,
            Error::MissingDep { .. } => 5,
            Error::ModelDownload(_) | Error::ModelMissing(_) => 6,
            _ => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_match_spec() {
        assert_eq!(Error::BadUrl("x".into()).exit_code(), 2);
        assert_eq!(Error::Download("x".into()).exit_code(), 3);
        assert_eq!(Error::Transcribe("x".into()).exit_code(), 4);
        assert_eq!(
            Error::MissingDep {
                name: "yt-dlp".into(),
                hint: "brew install yt-dlp".into()
            }
            .exit_code(),
            5
        );
        assert_eq!(Error::ModelDownload("x".into()).exit_code(), 6);
        assert_eq!(Error::Unsupported("x".into()).exit_code(), 2);
        assert_eq!(Error::Other("x".into()).exit_code(), 1);
    }

    #[test]
    fn unsupported_has_no_prefix() {
        // Must not masquerade as a yt-dlp failure.
        let msg = format!("{}", Error::Unsupported("no audio".into()));
        assert_eq!(msg, "no audio");
    }

    #[test]
    fn model_missing_has_no_download_prefix() {
        // The user hasn't downloaded a model; phrasing it as "download
        // failed" misdiagnoses the problem.
        let msg = format!(
            "{}",
            Error::ModelMissing("model small.en not installed".into())
        );
        assert_eq!(msg, "model small.en not installed");
    }

    #[test]
    fn model_missing_exits_6() {
        assert_eq!(Error::ModelMissing("x".into()).exit_code(), 6);
    }

    #[test]
    fn bad_arg_exits_2() {
        assert_eq!(Error::BadArg("x".into()).exit_code(), 2);
    }
}
