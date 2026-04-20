use crate::format::Format;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "tscribe",
    version,
    about = "Transcribe any video/audio URL into markdown"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// URL to transcribe (default mode, no subcommand needed).
    pub url: Option<String>,

    #[command(flatten)]
    pub transcribe: TranscribeArgs,
}

#[derive(Debug, clap::Args)]
pub struct TranscribeArgs {
    /// Write transcript to FILE instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Output format.
    #[arg(short, long, default_value = "md")]
    pub format: Format,

    /// Whisper model. Default: small.en for en, small for other langs.
    #[arg(short, long)]
    pub model: Option<String>,

    /// Language code (ISO 639-1). Default: en.
    #[arg(short, long, env = "TSCRIBE_DEFAULT_LANG", default_value = "en")]
    pub lang: String,

    /// Include [MM:SS] markers in markdown output.
    #[arg(long)]
    pub timestamps: bool,

    /// Skip cache lookup and don't write.
    #[arg(long)]
    pub no_cache: bool,

    /// Skip cache lookup but overwrite cache entry.
    #[arg(long)]
    pub refresh: bool,

    /// Show yt-dlp/whisper output for debugging.
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress all progress output.
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Whisper thread count (default: physical cores).
    #[arg(long)]
    pub threads: Option<u32>,

    /// Fail instead of prompting to download a model.
    #[arg(long)]
    pub no_download: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage the transcript cache.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Manage downloaded whisper models.
    Models {
        #[command(subcommand)]
        action: ModelAction,
    },

    /// Diagnose installation: check yt-dlp, ffmpeg, models.
    Doctor,

    /// Generate shell completions.
    Completions { shell: clap_complete::Shell },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// List cached transcripts.
    List,
    /// Remove all cached transcripts.
    Clear,
    /// Print the cache directory path.
    Path,
}

#[derive(Subcommand, Debug)]
pub enum ModelAction {
    /// List downloaded models with sizes.
    List,
    /// Pre-download a specific model.
    Download { name: String },
    /// Remove all downloaded models.
    Clear,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_basic_url() {
        let cli = Cli::try_parse_from(["tscribe", "https://example.com/x"]).unwrap();
        assert_eq!(cli.url.as_deref(), Some("https://example.com/x"));
        assert_eq!(cli.transcribe.format, Format::Markdown);
        assert_eq!(cli.transcribe.lang, "en");
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_format_and_lang() {
        let cli = Cli::try_parse_from(["tscribe", "https://x", "-f", "json", "-l", "nl"]).unwrap();
        assert_eq!(cli.transcribe.format, Format::Json);
        assert_eq!(cli.transcribe.lang, "nl");
    }

    #[test]
    fn quiet_and_verbose_are_mutually_exclusive() {
        let res = Cli::try_parse_from(["tscribe", "https://x", "-q", "-v"]);
        assert!(res.is_err());
    }

    #[test]
    fn parses_subcommands() {
        let cli = Cli::try_parse_from(["tscribe", "doctor"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Doctor)));

        let cli = Cli::try_parse_from(["tscribe", "cache", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Cache {
                action: CacheAction::List
            })
        ));

        let cli = Cli::try_parse_from(["tscribe", "models", "download", "small"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Models {
                action: ModelAction::Download { .. }
            })
        ));
    }
}
