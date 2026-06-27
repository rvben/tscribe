use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Schema {
    pub clispec: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub global_args: Vec<Arg>,
    pub commands: Vec<Command>,
    pub errors: Vec<ErrorKind>,
}

#[derive(Debug, Serialize)]
pub struct Arg {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<&'static str>>,
}

#[derive(Debug, Serialize)]
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    pub mutating: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<Arg>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub output_fields: Vec<Field>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<Command>,
}

#[derive(Debug, Serialize)]
pub struct Field {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub ty: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ErrorKind {
    pub kind: &'static str,
    pub exit_code: u8,
    pub retryable: bool,
    pub description: &'static str,
}

/// The per-command `--format` override shared by the metadata commands
/// (doctor, cache list, models list). It overrides the global `--output-mode`
/// for that command only.
fn format_override_arg() -> Arg {
    Arg {
        name: "--format",
        ty: "string",
        required: false,
        // No default: when omitted, this command inherits the global
        // --output-mode, which is not the same as passing --format=auto.
        default: None,
        description: Some(
            "Output format for this command (auto/text/json). Overrides --output-mode.",
        ),
        enum_values: Some(vec!["auto", "text", "json"]),
    }
}

pub fn build() -> Schema {
    Schema {
        clispec: "0.2",
        name: "tscribe",
        version: env!("CARGO_PKG_VERSION"),
        description: "Transcribe any video/audio URL into agent-friendly markdown using whisper.cpp",
        global_args: vec![
            Arg {
                name: "--output-mode",
                ty: "string",
                required: false,
                default: Some(serde_json::Value::String("auto".into())),
                description: Some(
                    "Tool metadata output mode. Controls doctor, cache list, models list output. \
                     Distinct from -f/--format which selects transcript content format.",
                ),
                enum_values: Some(vec!["auto", "text", "json"]),
            },
            Arg {
                name: "--json",
                ty: "boolean",
                required: false,
                default: Some(serde_json::Value::Bool(false)),
                description: Some(
                    "Shorthand for --output-mode json. Forces JSON output for doctor, cache list, \
                     models list.",
                ),
                enum_values: None,
            },
            Arg {
                name: "--quiet",
                ty: "boolean",
                required: false,
                default: Some(serde_json::Value::Bool(false)),
                description: Some("Suppress all progress output (transcript commands)."),
                enum_values: None,
            },
            Arg {
                name: "--verbose",
                ty: "boolean",
                required: false,
                default: Some(serde_json::Value::Bool(false)),
                description: Some("Show yt-dlp/whisper output for debugging."),
                enum_values: None,
            },
        ],
        commands: vec![
            Command {
                name: "schema",
                description: "Emit a machine-readable description of this tool's interface (clispec v0.2).",
                mutating: false,
                args: vec![],
                output_fields: vec![
                    Field {
                        name: "clispec",
                        ty: "string",
                        description: Some("Spec version"),
                    },
                    Field {
                        name: "name",
                        ty: "string",
                        description: Some("Tool name"),
                    },
                    Field {
                        name: "version",
                        ty: "string",
                        description: Some("Tool version"),
                    },
                    Field {
                        name: "commands",
                        ty: "array",
                        description: Some("Command definitions"),
                    },
                    Field {
                        name: "errors",
                        ty: "array",
                        description: Some("Error kind definitions"),
                    },
                ],
                subcommands: vec![],
            },
            Command {
                name: "doctor",
                description: "Diagnose installation: check yt-dlp, ffmpeg, models.",
                mutating: false,
                args: vec![
                    Arg {
                        name: "--limit",
                        ty: "integer",
                        required: false,
                        default: Some(serde_json::Value::Number(100.into())),
                        description: Some("Maximum number of installed models to show."),
                        enum_values: None,
                    },
                    Arg {
                        name: "--offset",
                        ty: "integer",
                        required: false,
                        default: Some(serde_json::Value::Number(0.into())),
                        description: Some("Number of installed models to skip."),
                        enum_values: None,
                    },
                    Arg {
                        name: "--fields",
                        ty: "string",
                        required: false,
                        default: None,
                        description: Some(
                            "Comma-separated fields to include in JSON output \
                             (version,dependencies,models,cache_dir,model_dir).",
                        ),
                        enum_values: None,
                    },
                    format_override_arg(),
                ],
                output_fields: vec![
                    Field {
                        name: "version",
                        ty: "string",
                        description: Some("tscribe version"),
                    },
                    Field {
                        name: "dependencies",
                        ty: "array",
                        description: Some("System dependency check results"),
                    },
                    Field {
                        name: "models",
                        ty: "array",
                        description: Some("Installed whisper models (bounded by --limit/--offset)"),
                    },
                    Field {
                        name: "models_total",
                        ty: "integer",
                        description: Some("Total number of installed models"),
                    },
                    Field {
                        name: "cache_dir",
                        ty: "string",
                        description: Some("Cache directory path"),
                    },
                    Field {
                        name: "model_dir",
                        ty: "string",
                        description: Some("Model directory path"),
                    },
                ],
                subcommands: vec![],
            },
            Command {
                name: "cache",
                description: "Manage the transcript cache.",
                mutating: false,
                args: vec![],
                output_fields: vec![],
                subcommands: vec![
                    Command {
                        name: "list",
                        description: "List cached transcripts.",
                        mutating: false,
                        args: vec![
                            Arg {
                                name: "--limit",
                                ty: "integer",
                                required: false,
                                default: Some(serde_json::Value::Number(100.into())),
                                description: Some("Maximum number of entries to return."),
                                enum_values: None,
                            },
                            Arg {
                                name: "--offset",
                                ty: "integer",
                                required: false,
                                default: Some(serde_json::Value::Number(0.into())),
                                description: Some("Number of entries to skip."),
                                enum_values: None,
                            },
                            Arg {
                                name: "--fields",
                                ty: "string",
                                required: false,
                                default: None,
                                description: Some(
                                    "Comma-separated fields to include (key,url,model,language,date,title).",
                                ),
                                enum_values: None,
                            },
                            format_override_arg(),
                        ],
                        output_fields: vec![
                            Field {
                                name: "items",
                                ty: "array",
                                description: Some("Cache entries"),
                            },
                            Field {
                                name: "total",
                                ty: "integer",
                                description: Some("Total entries in cache"),
                            },
                            Field {
                                name: "limit",
                                ty: "integer",
                                description: Some("Requested limit"),
                            },
                            Field {
                                name: "offset",
                                ty: "integer",
                                description: Some("Requested offset"),
                            },
                        ],
                        subcommands: vec![],
                    },
                    Command {
                        name: "clear",
                        description: "Remove all cached transcripts.",
                        mutating: true,
                        args: vec![],
                        output_fields: vec![],
                        subcommands: vec![],
                    },
                    Command {
                        name: "path",
                        description: "Print the cache directory path.",
                        mutating: false,
                        args: vec![],
                        output_fields: vec![Field {
                            name: "path",
                            ty: "string",
                            description: Some("Absolute path to the cache directory"),
                        }],
                        subcommands: vec![],
                    },
                ],
            },
            Command {
                name: "models",
                description: "Manage downloaded whisper models.",
                mutating: false,
                args: vec![],
                output_fields: vec![],
                subcommands: vec![
                    Command {
                        name: "list",
                        description: "List available models and their installation status.",
                        mutating: false,
                        args: vec![format_override_arg()],
                        output_fields: vec![
                            Field {
                                name: "name",
                                ty: "string",
                                description: None,
                            },
                            Field {
                                name: "size_mb",
                                ty: "integer",
                                description: None,
                            },
                            Field {
                                name: "installed",
                                ty: "boolean",
                                description: None,
                            },
                            Field {
                                name: "multilingual",
                                ty: "boolean",
                                description: Some(
                                    "Whether the model supports languages other than English",
                                ),
                            },
                        ],
                        subcommands: vec![],
                    },
                    Command {
                        name: "download",
                        description: "Pre-download a specific whisper model.",
                        mutating: true,
                        args: vec![Arg {
                            name: "name",
                            ty: "string",
                            required: true,
                            default: None,
                            description: Some(
                                "Model name (tiny.en, base.en, small.en, medium.en, small, large-v3).",
                            ),
                            enum_values: Some(vec![
                                "tiny.en",
                                "base.en",
                                "small.en",
                                "medium.en",
                                "small",
                                "large-v3",
                            ]),
                        }],
                        output_fields: vec![],
                        subcommands: vec![],
                    },
                    Command {
                        name: "clear",
                        description: "Remove all downloaded whisper models.",
                        mutating: true,
                        args: vec![],
                        output_fields: vec![],
                        subcommands: vec![],
                    },
                ],
            },
            Command {
                name: "completions",
                description: "Generate shell completions.",
                mutating: false,
                args: vec![Arg {
                    name: "shell",
                    ty: "string",
                    required: true,
                    default: None,
                    description: Some("Shell to generate completions for."),
                    enum_values: Some(vec!["bash", "fish", "zsh", "powershell", "elvish"]),
                }],
                output_fields: vec![],
                subcommands: vec![],
            },
            Command {
                name: "<url>",
                description: "Transcribe a video/audio URL into the selected format. This is the default mode; no subcommand is needed.",
                mutating: false,
                args: vec![
                    Arg {
                        name: "url",
                        ty: "string",
                        required: true,
                        default: None,
                        description: Some(
                            "URL of the video or audio to transcribe (any yt-dlp-supported source).",
                        ),
                        enum_values: None,
                    },
                    Arg {
                        name: "-f/--format",
                        ty: "string",
                        required: false,
                        default: Some(serde_json::Value::String("md".into())),
                        description: Some("Transcript content format."),
                        enum_values: Some(vec!["md", "txt", "json", "srt", "vtt"]),
                    },
                    Arg {
                        name: "-o/--output",
                        ty: "path",
                        required: false,
                        default: None,
                        description: Some("Write transcript to FILE instead of stdout."),
                        enum_values: None,
                    },
                    Arg {
                        name: "-m/--model",
                        ty: "string",
                        required: false,
                        default: None,
                        description: Some(
                            "Whisper model name. Default: small.en for English, small for other languages.",
                        ),
                        enum_values: Some(vec![
                            "tiny.en",
                            "base.en",
                            "small.en",
                            "medium.en",
                            "small",
                            "large-v3",
                        ]),
                    },
                    Arg {
                        name: "-l/--lang",
                        ty: "string",
                        required: false,
                        default: Some(serde_json::Value::String("en".into())),
                        description: Some("Language code (ISO 639-1)."),
                        enum_values: None,
                    },
                    Arg {
                        name: "--timestamps",
                        ty: "boolean",
                        required: false,
                        default: Some(serde_json::Value::Bool(false)),
                        description: Some("Include [MM:SS] markers in markdown output."),
                        enum_values: None,
                    },
                    Arg {
                        name: "--no-cache",
                        ty: "boolean",
                        required: false,
                        default: Some(serde_json::Value::Bool(false)),
                        description: Some("Skip cache lookup and don't write result to cache."),
                        enum_values: None,
                    },
                    Arg {
                        name: "--refresh",
                        ty: "boolean",
                        required: false,
                        default: Some(serde_json::Value::Bool(false)),
                        description: Some(
                            "Skip cache lookup but overwrite cache entry with new result.",
                        ),
                        enum_values: None,
                    },
                    Arg {
                        name: "--no-download",
                        ty: "boolean",
                        required: false,
                        default: Some(serde_json::Value::Bool(false)),
                        description: Some("Fail instead of auto-downloading a missing model."),
                        enum_values: None,
                    },
                    Arg {
                        name: "--threads",
                        ty: "integer",
                        required: false,
                        default: None,
                        description: Some(
                            "Whisper thread count. Default: physical CPU core count.",
                        ),
                        enum_values: None,
                    },
                ],
                output_fields: vec![Field {
                    name: "transcript",
                    ty: "string",
                    description: Some(
                        "The transcript content in the requested format, written to stdout.",
                    ),
                }],
                subcommands: vec![],
            },
        ],
        errors: vec![
            ErrorKind {
                kind: "invalid_url",
                exit_code: 2,
                retryable: false,
                description: "The provided URL is unsupported or malformed.",
            },
            ErrorKind {
                kind: "invalid_argument",
                exit_code: 2,
                retryable: false,
                description: "A flag or argument value is invalid (e.g. unknown model name or format).",
            },
            ErrorKind {
                kind: "network_failure",
                exit_code: 3,
                retryable: true,
                description: "Audio download failed (yt-dlp error or network issue).",
            },
            ErrorKind {
                kind: "transcription_failed",
                exit_code: 4,
                retryable: false,
                description: "Whisper transcription process failed.",
            },
            ErrorKind {
                kind: "missing_dependency",
                exit_code: 5,
                retryable: false,
                description: "A required system dependency (yt-dlp or ffmpeg) is not installed.",
            },
            ErrorKind {
                kind: "model_not_found",
                exit_code: 6,
                retryable: false,
                description: "The requested whisper model is not installed and --no-download was set, or model download failed.",
            },
            ErrorKind {
                kind: "io_error",
                exit_code: 1,
                retryable: false,
                description: "An unexpected I/O or internal error occurred.",
            },
            ErrorKind {
                kind: "conflict",
                exit_code: 7,
                retryable: false,
                description: "A resource exists with a configuration that conflicts with the requested state.",
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_serializes_to_valid_json() {
        let schema = build();
        let json = serde_json::to_string_pretty(&schema).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(value["clispec"], "0.2");
        assert_eq!(value["name"], "tscribe");
        assert!(value["commands"].is_array());
        assert!(value["errors"].is_array());
        assert!(value["global_args"].is_array());
    }

    #[test]
    fn schema_has_required_fields() {
        let schema = build();
        assert!(!schema.commands.is_empty());
        assert!(!schema.errors.is_empty());
        assert!(!schema.global_args.is_empty());
    }

    #[test]
    fn all_errors_have_exit_codes() {
        let schema = build();
        for err in &schema.errors {
            assert!(
                err.exit_code > 0,
                "error kind '{}' has exit_code 0",
                err.kind
            );
        }
    }

    #[test]
    fn subcommand_names_are_leaf_tokens() {
        // A consumer derives the full invocation by joining a parent command's
        // name with each subcommand's name. So a subcommand `name` must be the
        // leaf token only ("list"), never the already-joined path ("cache
        // list") — otherwise the derived path duplicates the noun
        // ("cache cache list"), which both misleads agents and breaks tooling
        // that discovers a representative command from the schema.
        fn check(commands: &[Command]) {
            for cmd in commands {
                for sub in &cmd.subcommands {
                    assert!(
                        !sub.name.contains(' '),
                        "subcommand '{}' under '{}' must be a leaf token, not include the parent prefix",
                        sub.name,
                        cmd.name
                    );
                }
                check(&cmd.subcommands);
            }
        }
        check(&build().commands);
    }

    #[test]
    fn all_commands_have_mutating_field() {
        let schema = build();
        fn check(commands: &[Command]) {
            for cmd in commands {
                // name field is always present via struct
                let _ = cmd.mutating;
                check(&cmd.subcommands);
            }
        }
        check(&schema.commands);
    }

    #[test]
    fn schema_validates_against_clispec_v02() {
        let schema_doc = build();
        let json = serde_json::to_value(&schema_doc).unwrap();

        // Load the vendored JSON Schema and validate structurally.
        let meta_schema: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/clispec-v0.2.json"))
                .expect("parse vendored schema");

        // Verify required top-level properties per the JSON Schema.
        for required in meta_schema["required"].as_array().unwrap() {
            let key = required.as_str().unwrap();
            assert!(
                !json[key].is_null(),
                "schema output missing required field: {key}"
            );
        }

        // Validate that commands array is non-empty.
        assert!(
            json["commands"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "commands array must be non-empty"
        );

        // Validate clispec version pattern.
        let clispec = json["clispec"].as_str().unwrap_or("");
        assert!(
            clispec.contains('.'),
            "clispec version must match N.N pattern, got: {clispec}"
        );

        // Validate each error has 'kind'.
        for err in json["errors"].as_array().unwrap() {
            assert!(err["kind"].is_string(), "error missing kind field");
        }
    }
}
