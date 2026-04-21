use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptEntry {
    pub version: u32,
    pub url: String,
    pub metadata: Metadata,
    pub transcription: Transcription,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub site: Option<String>,
    pub duration_seconds: Option<u64>,
    pub uploaded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transcription {
    pub model: String,
    pub language: String,
    pub transcribed_at: DateTime<Utc>,
    pub tscribe_version: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

impl TranscriptEntry {
    /// Total duration as reported by metadata, or last segment end as fallback.
    pub fn duration_seconds(&self) -> f64 {
        if let Some(d) = self.metadata.duration_seconds {
            return d as f64;
        }
        self.transcription
            .segments
            .last()
            .map(|s| s.end)
            .unwrap_or(0.0)
    }
}

impl Metadata {
    /// One-line summary matching the format used during probing:
    /// `"Title" — Author (3m20s)`. Any missing part is omitted.
    pub fn summary(&self) -> String {
        let mut out = self
            .title
            .as_deref()
            .map(|t| format!("\"{t}\""))
            .unwrap_or_else(|| "(untitled)".to_string());
        if let Some(author) = &self.author {
            out.push_str(" — ");
            out.push_str(author);
        }
        if let Some(secs) = self.duration_seconds {
            let m = secs / 60;
            let s = secs % 60;
            if m > 0 {
                out.push_str(&format!(" ({m}m{s:02}s)"));
            } else {
                out.push_str(&format!(" ({s}s)"));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/transcript.json");

    #[test]
    fn fixture_round_trips() {
        let entry: TranscriptEntry = serde_json::from_str(FIXTURE).expect("parse fixture");
        assert_eq!(entry.version, SCHEMA_VERSION);
        assert_eq!(entry.transcription.segments.len(), 3);
        assert_eq!(entry.duration_seconds(), 12.0);

        let serialized = serde_json::to_string_pretty(&entry).unwrap();
        let reparsed: TranscriptEntry = serde_json::from_str(&serialized).unwrap();
        assert_eq!(entry, reparsed);
    }
}
