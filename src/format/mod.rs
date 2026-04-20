use crate::transcript::TranscriptEntry;
use std::fmt;

pub mod json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Markdown,
    Text,
    Json,
    Srt,
    Vtt,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Markdown => "md",
            Format::Text => "txt",
            Format::Json => "json",
            Format::Srt => "srt",
            Format::Vtt => "vtt",
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

impl std::str::FromStr for Format {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "md" | "markdown" => Ok(Format::Markdown),
            "txt" | "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            "srt" => Ok(Format::Srt),
            "vtt" => Ok(Format::Vtt),
            other => Err(format!("unknown format: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
    pub timestamps: bool,
}

pub fn render(entry: &TranscriptEntry, format: Format, _opts: RenderOptions) -> String {
    match format {
        Format::Json => json::render(entry),
        // Other variants added in later tasks.
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parses_format_aliases() {
        assert_eq!(Format::from_str("md").unwrap(), Format::Markdown);
        assert_eq!(Format::from_str("MARKDOWN").unwrap(), Format::Markdown);
        assert_eq!(Format::from_str("text").unwrap(), Format::Text);
        assert_eq!(Format::from_str("vtt").unwrap(), Format::Vtt);
        assert!(Format::from_str("xml").is_err());
    }
}
