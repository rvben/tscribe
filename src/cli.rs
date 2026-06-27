use crate::format::Format;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;

/// Output mode for tool metadata commands (doctor, cache list, models list).
/// Controls whether status and diagnostic output is human text or structured JSON.
/// Distinct from `-f`/`--format` which selects the transcript content format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Human-friendly text when stdout is a TTY, JSON when piped.
    #[default]
    Auto,
    /// Always human-friendly text.
    Text,
    /// Always structured JSON.
    Json,
}

impl FromStr for OutputMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(OutputMode::Auto),
            "text" => Ok(OutputMode::Text),
            "json" => Ok(OutputMode::Json),
            other => Err(format!(
                "unknown output mode: {other} (valid: auto, text, json)"
            )),
        }
    }
}

impl OutputMode {
    /// Resolve auto mode using TTY detection.
    pub fn resolve(self) -> ResolvedOutputMode {
        match self {
            OutputMode::Auto => {
                use std::io::IsTerminal;
                if std::io::stdout().is_terminal() {
                    ResolvedOutputMode::Text
                } else {
                    ResolvedOutputMode::Json
                }
            }
            OutputMode::Text => ResolvedOutputMode::Text,
            OutputMode::Json => ResolvedOutputMode::Json,
        }
    }

    /// Merge a `--json` boolean override: if `json_flag` is true, force Json mode.
    pub fn with_json_flag(self, json_flag: bool) -> Self {
        if json_flag { OutputMode::Json } else { self }
    }
}

/// The resolved (non-auto) output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedOutputMode {
    Text,
    Json,
}

#[derive(Parser, Debug)]
#[command(
    name = "tscribe",
    version,
    about = "Transcribe any video/audio URL into markdown. Run `tscribe schema` for machine-readable interface description."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// URL to transcribe (default mode, no subcommand needed).
    pub url: Option<String>,

    /// Tool metadata output mode (auto/text/json). Controls doctor, cache list, models list output.
    /// Distinct from -f/--format which selects transcript content format.
    #[arg(long, default_value = "auto", global = true, value_name = "MODE")]
    pub output_mode: OutputMode,

    /// Shorthand for --output-mode json. Forces JSON output for doctor, cache list, models list.
    #[arg(long, global = true, hide = false)]
    pub json: bool,

    #[command(flatten)]
    pub transcribe: TranscribeArgs,
}

#[derive(Debug, clap::Args)]
pub struct TranscribeArgs {
    /// Write transcript to FILE instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Transcript content format (md|txt|json|srt|vtt).
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
    Doctor {
        /// Maximum number of installed models to show.
        #[arg(long, default_value = "100")]
        limit: usize,
        /// Number of installed models to skip.
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Comma-separated fields to include in JSON output (version,dependencies,models,cache_dir,model_dir).
        #[arg(long)]
        fields: Option<String>,
        /// Output format for this command (auto/text/json). Overrides --output-mode for doctor.
        #[arg(long, value_name = "FORMAT")]
        format: Option<OutputMode>,
    },

    /// Generate shell completions.
    Completions { shell: clap_complete::Shell },

    /// Emit a machine-readable description of this tool's interface (clispec v0.2).
    Schema,
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// List cached transcripts.
    List {
        /// Maximum number of entries to return.
        #[arg(long, default_value = "100")]
        limit: usize,
        /// Number of entries to skip.
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Comma-separated fields to include (key,url,model,language,date,title).
        #[arg(long)]
        fields: Option<String>,
        /// Output format for this command (auto/text/json). Overrides --output-mode.
        #[arg(long, value_name = "FORMAT")]
        format: Option<OutputMode>,
    },
    /// Remove all cached transcripts.
    Clear,
    /// Print the cache directory path.
    Path,
}

#[derive(Subcommand, Debug)]
pub enum ModelAction {
    /// List downloaded models with sizes.
    List {
        /// Output format for this command (auto/text/json). Overrides --output-mode.
        #[arg(long, value_name = "FORMAT")]
        format: Option<OutputMode>,
    },
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
        assert!(matches!(cli.command, Some(Command::Doctor { .. })));

        let cli = Cli::try_parse_from(["tscribe", "cache", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Cache {
                action: CacheAction::List { .. }
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

    #[test]
    fn parses_schema_subcommand() {
        let cli = Cli::try_parse_from(["tscribe", "schema"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Schema)));
    }

    #[test]
    fn output_mode_defaults_to_auto() {
        let cli = Cli::try_parse_from(["tscribe", "doctor"]).unwrap();
        assert_eq!(cli.output_mode, OutputMode::Auto);
        assert!(!cli.json);
    }

    #[test]
    fn output_mode_parses_json() {
        let cli = Cli::try_parse_from(["tscribe", "--output-mode", "json", "doctor"]).unwrap();
        assert_eq!(cli.output_mode, OutputMode::Json);
    }

    #[test]
    fn json_flag_forces_json_mode() {
        let cli = Cli::try_parse_from(["tscribe", "--json", "doctor"]).unwrap();
        assert!(cli.json);
        assert_eq!(
            cli.output_mode.with_json_flag(cli.json).resolve(),
            ResolvedOutputMode::Json
        );
    }

    #[test]
    fn cache_list_parses_limit_offset() {
        let cli =
            Cli::try_parse_from(["tscribe", "cache", "list", "--limit", "10", "--offset", "5"])
                .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Cache {
                action: CacheAction::List {
                    limit: 10,
                    offset: 5,
                    ..
                }
            })
        ));
    }

    #[test]
    fn doctor_parses_limit_offset_fields() {
        let cli = Cli::try_parse_from([
            "tscribe",
            "doctor",
            "--limit",
            "5",
            "--offset",
            "1",
            "--fields",
            "version,models",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Doctor {
                limit: 5,
                offset: 1,
                ..
            })
        ));
    }

    #[test]
    fn cache_list_parses_format_override() {
        let cli = Cli::try_parse_from(["tscribe", "cache", "list", "--format", "text"]).unwrap();
        match cli.command {
            Some(Command::Cache {
                action: CacheAction::List { format, .. },
            }) => assert_eq!(format, Some(OutputMode::Text)),
            _ => panic!("expected cache list"),
        }
    }

    #[test]
    fn models_list_parses_format_override() {
        let cli = Cli::try_parse_from(["tscribe", "models", "list", "--format", "json"]).unwrap();
        match cli.command {
            Some(Command::Models {
                action: ModelAction::List { format },
            }) => assert_eq!(format, Some(OutputMode::Json)),
            _ => panic!("expected models list"),
        }
    }

    #[test]
    fn output_mode_from_str() {
        assert_eq!(OutputMode::from_str("auto").unwrap(), OutputMode::Auto);
        assert_eq!(OutputMode::from_str("json").unwrap(), OutputMode::Json);
        assert_eq!(OutputMode::from_str("text").unwrap(), OutputMode::Text);
        assert!(OutputMode::from_str("yaml").is_err());
    }
}
